/* based on
   https://discourse.nixos.org/t/how-can-i-set-up-my-rust-programming-environment/4501/9
*/
let
  rust_overlay = import (builtins.fetchTarball
    "https://github.com/oxalica/rust-overlay/archive/master.tar.gz");
  pkgs = import <nixpkgs> { overlays = [ rust_overlay ]; };
  rustVersion = "latest";
  #rustVersion = "1.62.0";
  rust = pkgs.rust-bin.stable.${rustVersion}.default.override {
    extensions = [
      "rust-src" # for rust-analyzer
      "rust-analyzer"
    ];
  };
in pkgs.mkShell {
  buildInputs = [ rust ] ++ (with pkgs; [
    pkg-config
    protobuf_28
    # other dependencies
    #gtk3
    #wrapGAppsHook
  ]);
  RUST_BACKTRACE = 1;
}
