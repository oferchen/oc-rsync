class OcRsync < Formula
  desc "Pure-Rust rsync replica"
  homepage "https://github.com/oferchen/oc-rsync"
  url "https://github.com/oferchen/oc-rsync/archive/refs/heads/main.tar.gz"
  sha256 "c5f90968dd721c6c062b7441227d3f5582b40fc2e21b90243ee2d896d7f19abf"
  license "Apache-2.0"
  head "https://github.com/oferchen/oc-rsync.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "."), "--bin", "oc-rsync", "--bin", "oc-rsyncd"
    man1.install "man/oc-rsync.1"
    man8.install "man/oc-rsyncd.8"
    man5.install "man/oc-rsyncd.conf.5"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/oc-rsync --version")
  end
end
