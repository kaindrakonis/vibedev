/**
 * Postinstall binary downloader for claudev - Optimized C++ implementation
 *
 * Build: g++ -std=c++17 -O3 -o postinstall postinstall.cpp -lcurl -lz -larchive
 * Or:    clang++ -std=c++17 -O3 -o postinstall postinstall.cpp -lcurl -lz -larchive
 */

#include <iostream>
#include <fstream>
#include <vector>
#include <string>
#include <string_view>
#include <optional>
#include <filesystem>
#include <memory>
#include <system_error>
#include <cstring>
#include <curl/curl.h>
#include <archive.h>
#include <archive_entry.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <sys/stat.h>
#include <unistd.h>
#endif

namespace fs = std::filesystem;

// Configuration constants
constexpr std::string_view REPO = "openSVM/vibedev";
constexpr std::string_view BINARY_NAME = "claudev";
constexpr size_t DOWNLOAD_TIMEOUT = 60;
constexpr size_t MAX_REDIRECTS = 5;
constexpr size_t BUFFER_SIZE = 8192;

// Platform detection
struct Platform {
    std::string os;
    std::string arch;
    std::string target_triple;
    std::string archive_ext;

    static Platform detect() noexcept {
        Platform p;

#ifdef _WIN32
        p.os = "win32";
        p.archive_ext = "zip";
#elif __APPLE__
        p.os = "darwin";
        p.archive_ext = "tar.gz";
#elif __linux__
        p.os = "linux";
        p.archive_ext = "tar.gz";
#else
        p.os = "unknown";
#endif

#if defined(__x86_64__) || defined(_M_X64)
        p.arch = "x64";
#elif defined(__aarch64__) || defined(_M_ARM64)
        p.arch = "arm64";
#else
        p.arch = "unknown";
#endif

        // Map to Rust target triple
        if (p.os == "darwin" && p.arch == "x64") {
            p.target_triple = "x86_64-apple-darwin";
        } else if (p.os == "darwin" && p.arch == "arm64") {
            p.target_triple = "aarch64-apple-darwin";
        } else if (p.os == "linux" && p.arch == "x64") {
            p.target_triple = "x86_64-unknown-linux-gnu";
        } else if (p.os == "win32" && p.arch == "x64") {
            p.target_triple = "x86_64-pc-windows-msvc";
        }

        return p;
    }

    [[nodiscard]] bool is_supported() const noexcept {
        return !target_triple.empty();
    }

    [[nodiscard]] std::string get_binary_name() const {
        return std::string(BINARY_NAME) + (os == "win32" ? ".exe" : "");
    }
};

// RAII wrapper for CURL
class CurlHandle {
    CURL* handle_;

public:
    CurlHandle() : handle_(curl_easy_init()) {
        if (!handle_) {
            throw std::runtime_error("Failed to initialize CURL");
        }
    }

    ~CurlHandle() {
        if (handle_) curl_easy_cleanup(handle_);
    }

    // No copy, only move
    CurlHandle(const CurlHandle&) = delete;
    CurlHandle& operator=(const CurlHandle&) = delete;

    CurlHandle(CurlHandle&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }

    CURL* get() noexcept { return handle_; }
};

// Download callback for libcurl
struct DownloadBuffer {
    std::vector<uint8_t> data;
    size_t last_reported_mb = 0;

    static size_t write_callback(void* contents, size_t size, size_t nmemb, void* userp) {
        size_t total_size = size * nmemb;
        auto* buffer = static_cast<DownloadBuffer*>(userp);

        const auto* bytes = static_cast<const uint8_t*>(contents);
        buffer->data.insert(buffer->data.end(), bytes, bytes + total_size);

        // Progress indicator (print every MB)
        size_t current_mb = buffer->data.size() / (1024 * 1024);
        if (current_mb > buffer->last_reported_mb) {
            std::cout << "\r  Downloaded " << current_mb << " MB" << std::flush;
            buffer->last_reported_mb = current_mb;
        }

        return total_size;
    }
};

// Download file with redirect support
std::optional<std::vector<uint8_t>> download_file(std::string_view url) {
    try {
        CurlHandle curl;
        DownloadBuffer buffer;

        curl_easy_setopt(curl.get(), CURLOPT_URL, url.data());
        curl_easy_setopt(curl.get(), CURLOPT_FOLLOWLOCATION, 1L);
        curl_easy_setopt(curl.get(), CURLOPT_MAXREDIRS, MAX_REDIRECTS);
        curl_easy_setopt(curl.get(), CURLOPT_TIMEOUT, DOWNLOAD_TIMEOUT);
        curl_easy_setopt(curl.get(), CURLOPT_WRITEFUNCTION, DownloadBuffer::write_callback);
        curl_easy_setopt(curl.get(), CURLOPT_WRITEDATA, &buffer);
        curl_easy_setopt(curl.get(), CURLOPT_USERAGENT, "claudev-installer/cpp");
        curl_easy_setopt(curl.get(), CURLOPT_SSL_VERIFYPEER, 1L);

        const CURLcode res = curl_easy_perform(curl.get());

        if (res != CURLE_OK) {
            std::cerr << "\nDownload failed: " << curl_easy_strerror(res) << std::endl;
            return std::nullopt;
        }

        long response_code;
        curl_easy_getinfo(curl.get(), CURLINFO_RESPONSE_CODE, &response_code);

        if (response_code != 200) {
            std::cerr << "\nHTTP " << response_code << std::endl;
            return std::nullopt;
        }

        std::cout << " - done" << std::endl;
        return std::move(buffer.data);

    } catch (const std::exception& e) {
        std::cerr << "\nDownload exception: " << e.what() << std::endl;
        return std::nullopt;
    }
}

// Extract archive using libarchive (handles both tar.gz and zip)
bool extract_archive(const std::vector<uint8_t>& data, const fs::path& dest_dir) {
    struct archive* a = archive_read_new();
    struct archive* ext = archive_write_disk_new();

    if (!a || !ext) {
        std::cerr << "Failed to create archive handles" << std::endl;
        return false;
    }

    // Enable all formats and filters
    archive_read_support_format_all(a);
    archive_read_support_filter_all(a);

    // Set extraction flags
    int flags = ARCHIVE_EXTRACT_TIME | ARCHIVE_EXTRACT_PERM | ARCHIVE_EXTRACT_ACL | ARCHIVE_EXTRACT_FFLAGS;
    archive_write_disk_set_options(ext, flags);
    archive_write_disk_set_standard_lookup(ext);

    // Open archive from memory
    int r = archive_read_open_memory(a, data.data(), data.size());
    if (r != ARCHIVE_OK) {
        std::cerr << "Failed to open archive: " << archive_error_string(a) << std::endl;
        archive_read_free(a);
        archive_write_free(ext);
        return false;
    }

    // Extract all entries
    struct archive_entry* entry;
    bool success = true;

    while (archive_read_next_header(a, &entry) == ARCHIVE_OK) {
        // Construct full path
        const char* current_file = archive_entry_pathname(entry);
        fs::path full_path = dest_dir / current_file;

        archive_entry_set_pathname(entry, full_path.c_str());

        r = archive_write_header(ext, entry);
        if (r != ARCHIVE_OK) {
            std::cerr << "Write header failed: " << archive_error_string(ext) << std::endl;
            success = false;
            break;
        }

        // Copy data
        const void* buff;
        size_t size;
        int64_t offset;

        while ((r = archive_read_data_block(a, &buff, &size, &offset)) == ARCHIVE_OK) {
            if (archive_write_data_block(ext, buff, size, offset) != ARCHIVE_OK) {
                std::cerr << "Write data failed: " << archive_error_string(ext) << std::endl;
                success = false;
                break;
            }
        }

        if (r != ARCHIVE_EOF) {
            std::cerr << "Read data failed: " << archive_error_string(a) << std::endl;
            success = false;
            break;
        }

        r = archive_write_finish_entry(ext);
        if (r != ARCHIVE_OK) {
            std::cerr << "Finish entry failed: " << archive_error_string(ext) << std::endl;
            success = false;
            break;
        }
    }

    archive_read_close(a);
    archive_read_free(a);
    archive_write_close(ext);
    archive_write_free(ext);

    return success;
}

// Set executable permissions (Unix only)
bool make_executable(const fs::path& file_path) {
#ifndef _WIN32
    return chmod(file_path.c_str(), S_IRWXU | S_IRGRP | S_IXGRP | S_IROTH | S_IXOTH) == 0;
#else
    return true; // Windows doesn't need this
#endif
}

// Read package version from package.json
std::string get_package_version() {
    fs::path package_json = fs::path(__FILE__).parent_path().parent_path() / "package.json";

    std::ifstream file(package_json);
    if (!file.is_open()) {
        return "0.5.0"; // Fallback version
    }

    std::string line;
    while (std::getline(file, line)) {
        if (line.find("\"version\"") != std::string::npos) {
            size_t start = line.find('"', line.find(':')) + 1;
            size_t end = line.find('"', start);
            return line.substr(start, end - start);
        }
    }

    return "0.5.0";
}

int main() {
    std::cout << "claudev: Installing binary..." << std::endl;

    // Initialize libcurl globally
    curl_global_init(CURL_GLOBAL_DEFAULT);

    // Detect platform
    const Platform platform = Platform::detect();

    if (!platform.is_supported()) {
        std::cerr << "Unsupported platform: " << platform.os << "-" << platform.arch << std::endl;
        std::cerr << "Build from source: cargo install claudev" << std::endl;
        curl_global_cleanup();
        return 0; // Don't fail npm install
    }

    // Setup paths
    const fs::path script_dir = fs::path(__FILE__).parent_path();
    const fs::path bin_dir = script_dir.parent_path() / "bin";
    const fs::path binary_path = bin_dir / platform.get_binary_name();

    // Check if binary already exists
    if (fs::exists(binary_path)) {
        std::cout << "claudev: Binary exists, skipping download" << std::endl;
        curl_global_cleanup();
        return 0;
    }

    // Build download URL
    const std::string version = get_package_version();
    const std::string url = std::string("https://github.com/") +
                           std::string(REPO) +
                           "/releases/download/v" + version +
                           "/" + std::string(BINARY_NAME) +
                           "-" + platform.target_triple +
                           "." + platform.archive_ext;

    std::cout << "claudev: Downloading for " << platform.os << "-" << platform.arch << std::endl;

    // Download archive
    auto download_result = download_file(url);
    if (!download_result) {
        std::cerr << "claudev: Download failed" << std::endl;
        std::cerr << "claudev: Install manually: cargo install claudev" << std::endl;
        curl_global_cleanup();
        return 0; // Don't fail npm install
    }

    // Create bin directory
    std::error_code ec;
    fs::create_directories(bin_dir, ec);
    if (ec) {
        std::cerr << "Failed to create bin directory: " << ec.message() << std::endl;
        curl_global_cleanup();
        return 1;
    }

    // Extract archive
    if (!extract_archive(*download_result, bin_dir)) {
        std::cerr << "claudev: Extraction failed" << std::endl;
        curl_global_cleanup();
        return 1;
    }

    // Make executable (Unix)
    if (platform.os != "win32") {
        if (!make_executable(binary_path)) {
            std::cerr << "Warning: Failed to set executable permissions" << std::endl;
        }
    }

    std::cout << "claudev: Installed successfully!" << std::endl;

    curl_global_cleanup();
    return 0;
}
