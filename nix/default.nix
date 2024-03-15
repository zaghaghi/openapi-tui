{
  system,
  pkgs,
  lockFile,
  fenix,
}: let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  toolchain = fenix.packages.${system}.minimal.toolchain;
in
  (pkgs.makeRustPlatform {
    cargo = toolchain;
    rustc = toolchain;
  })
  .buildRustPackage rec {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;

    src = ../.;

    cargoLock = {
      lockFile = lockFile;
    };

    nativeBuildInputs = with pkgs; [
      pkg-config
      makeWrapper
      rustfmt
    ];

    doCheck = true;
    CARGO_BUILD_INCREMENTAL = "false";
    RUST_BACKTRACE = "full";
    copyLibs = true;

    postInstall = ''
      wrapProgram $out/bin/${pname}
    '';

    meta = with pkgs.lib; {
      homepage = "https://github.com/zaghaghi/openapi-tui";
      description = "Terminal UI to list, browse and run APIs defined with OpenAPI v3.0 spec";
      license = licenses.mit;
      platforms = platforms.linux;
      mainProgram = pname;
    };
  }
