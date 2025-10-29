{ pkgs, ... }: {
  # Which nixpkgs channel to use.
  channel = "unstable"; # or "unstable"
  # Use https://search.nixos.org/packages to find packages
  packages = [
    pkgs.rustup
    pkgs.musl
    pkgs.fish
    pkgs.fastfetch
  ];
  # Sets environment variables in the workspace
  env = {
    RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
  };
  idx = {
    # Search for the extensions you want on https://open-vsx.org/ and use "publisher.id"
    extensions = [
      "rust-lang.rust-analyzer"
      "tamasfe.even-better-toml"
      "serayuzgur.crates"
      "vadimcn.vscode-lldb"
    ];
    workspace = {
      onCreate = {
        # rust-init = "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh && rustup target add x86_64-unknown-linux-musl";
        default.openFiles = ["src/main.rs"];
      };
    };
    # Enable previews and customize configuration
    previews = {};
  };
}