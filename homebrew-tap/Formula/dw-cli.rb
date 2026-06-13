class DwCli < Formula
  desc "Rust-native DataWeave CLI"
  homepage "https://github.com/estebanwasinger/dataweave-py"
  url "https://github.com/estebanwasinger/dataweave-py/releases/download/v1.0.2/dw-cli-1.0.2-source.tar.gz"
  sha256 "2025a24f3399fbff11e8d2d961dafbb87d3f0d1723286e2a2c79263a0c747f7f"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/dwpy-cli")
  end

  test do
    output = shell_output("#{bin}/dw run \"1 to 1\"")
    assert_match "[", output
    assert_match "1", output
    assert_match "]", output
  end
end
