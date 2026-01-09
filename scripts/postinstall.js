#!/usr/bin/env node

/**
 * Postinstall script for claudev npm package
 * Downloads the appropriate prebuilt binary for the user's platform
 */

const https = require('https');
const fs = require('fs');
const path = require('path');
const os = require('os');
const { execFileSync } = require('child_process');

const PACKAGE_VERSION = require('../package.json').version;
const REPO = 'openSVM/vibedev';
const BINARY_NAME = 'claudev';
const DOWNLOAD_TIMEOUT = 60000; // 60 seconds

// Map Node.js platform/arch to Rust target triples
const PLATFORM_MAP = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc',
};

function getPlatformKey() {
  return `${os.platform()}-${os.arch()}`;
}

function getBinaryExtension() {
  return os.platform() === 'win32' ? '.exe' : '';
}

function getDownloadUrl(target) {
  const ext = os.platform() === 'win32' ? 'zip' : 'tar.gz';
  return `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/${BINARY_NAME}-${target}.${ext}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error('Download timed out'));
    }, DOWNLOAD_TIMEOUT);

    const request = (url, redirectCount = 0) => {
      if (redirectCount > 5) {
        clearTimeout(timeout);
        reject(new Error('Too many redirects'));
        return;
      }

      const req = https.get(url, { headers: { 'User-Agent': 'claudev-installer' } }, (response) => {
        if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          request(response.headers.location, redirectCount + 1);
          return;
        }

        if (response.statusCode !== 200) {
          clearTimeout(timeout);
          reject(new Error(`HTTP ${response.statusCode}`));
          return;
        }

        const chunks = [];
        let downloaded = 0;

        response.on('data', (chunk) => {
          chunks.push(chunk);
          downloaded += chunk.length;
          process.stdout.write(`\r  Downloaded ${(downloaded / 1024 / 1024).toFixed(1)} MB`);
        });

        response.on('end', () => {
          clearTimeout(timeout);
          console.log(' - done');
          resolve(Buffer.concat(chunks));
        });

        response.on('error', (err) => {
          clearTimeout(timeout);
          reject(err);
        });
      });

      req.on('error', (err) => {
        clearTimeout(timeout);
        reject(err);
      });
    };

    request(url);
  });
}

function extractTarGz(buffer, destDir) {
  const tarPath = path.join(destDir, 'temp.tar.gz');
  fs.writeFileSync(tarPath, buffer);
  try {
    execFileSync('tar', ['-xzf', tarPath, '-C', destDir], { stdio: 'pipe' });
  } finally {
    fs.unlinkSync(tarPath);
  }
}

function extractZip(buffer, destDir) {
  const zipPath = path.join(destDir, 'temp.zip');
  fs.writeFileSync(zipPath, buffer);
  try {
    execFileSync('powershell', ['-Command', `Expand-Archive -Path '${zipPath}' -DestinationPath '${destDir}' -Force`], { stdio: 'pipe' });
  } finally {
    fs.unlinkSync(zipPath);
  }
}

async function main() {
  console.log('claudev: Installing binary...');

  const platformKey = getPlatformKey();
  const target = PLATFORM_MAP[platformKey];

  if (!target) {
    console.error(`Unsupported platform: ${platformKey}`);
    console.error('Build from source: cargo install claudev');
    process.exit(0); // Don't fail install
  }

  const binDir = path.join(__dirname, '..', 'bin');
  const binaryPath = path.join(binDir, BINARY_NAME + getBinaryExtension());

  if (fs.existsSync(binaryPath)) {
    console.log('claudev: Binary exists, skipping download');
    return;
  }

  const downloadUrl = getDownloadUrl(target);
  console.log(`claudev: Downloading for ${platformKey}`);

  try {
    const buffer = await download(downloadUrl);

    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    if (os.platform() === 'win32') {
      extractZip(buffer, binDir);
    } else {
      extractTarGz(buffer, binDir);
    }

    if (os.platform() !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log('claudev: Installed successfully!');
  } catch (error) {
    console.error(`claudev: Download failed - ${error.message}`);
    console.error('claudev: Install manually: cargo install claudev');
    // Don't fail - let wrapper show error
  }
}

main().catch(console.error);
