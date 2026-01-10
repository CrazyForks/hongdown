{
  lib,
  rustPlatform,
  ...
}:
rustPlatform.buildRustPackage {
  name = "hongdown";
  version = "0.1.0";

  src = ./..;
  cargoLock.lockFile = ../Cargo.lock;

  meta = {
    description = "A Markdown formatter that enforces Hong Minhee's Markdown style conventions";
    mainProgram = "hongdown";
    homepage = "https://github.com/dahlia/hongdown";
    license = lib.licenses.gpl3;
    maintainers = [];
  };
}
