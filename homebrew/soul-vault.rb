# Homebrew formula for Soul Vault
# To install (once published to a tap):
#   brew install user/tap/soul-vault
#
# To test locally:
#   brew install --build-from-source ./homebrew/soul-vault.rb

class SoulVault < Formula
  desc "Your AI memory, unified — distills AI conversations into a structured local vault"
  homepage "https://github.com/user/soul-vault"
  license "MIT"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/user/soul-vault/releases/download/v#{version}/soul-macos-arm64"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"

      def install
        bin.install "soul-macos-arm64" => "soul"
      end
    else
      url "https://github.com/user/soul-vault/releases/download/v#{version}/soul-macos-x86_64"
      sha256 "PLACEHOLDER_SHA256_MACOS_X86_64"

      def install
        bin.install "soul-macos-x86_64" => "soul"
      end
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/user/soul-vault/releases/download/v#{version}/soul-linux-arm64"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"

      def install
        bin.install "soul-linux-arm64" => "soul"
      end
    else
      url "https://github.com/user/soul-vault/releases/download/v#{version}/soul-linux-x86_64"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"

      def install
        bin.install "soul-linux-x86_64" => "soul"
      end
    end
  end

  test do
    assert_match "soul", shell_output("#{bin}/soul --help")
  end
end
