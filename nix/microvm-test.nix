{ pkgs }:

let
  # Define a minimal NixOS configuration for the micro-vm
  vmConfig = {
    # Use a minimal base
    imports = [ (pkgs.lib.mkDefaultLib)/nixpkgs/nixos/modules/profiles/minimal.nix ];
    
    # Install rust and cargo
    environment.systemPackages = with pkgs; [
      cargo
      rustc
      git
      curl
      bash
    ];

    # Enable SSH for easy access or just use a console
    services.openssh.enable = true;
    services.openssh.settings.PermitRootLogin = "yes";
    
    # Set a password for root
    users.users.root.initialPassword = "root";
  };

  # This expression returns the configuration. 
  # To actually build a VM, one would typically use 'nixos-rebuild build-vm' 
  # or a tool like 'microvm.nix'.
in
pkgs.stdenv.mkDerivation {
  pname = "pyroduct-microvm-test-script";
  version = "0.1.0";
  src = ./..;
  phases = [ "installPhase" ];
  installPhase = ''
    mkdir -p $out/bin
    cat << 'EOF' > $out/bin/test-install-sh
    set -e
    echo "=== Starting Pyroduct Install Test ==="
    echo "This script describes the test process."
    echo "To execute this in a real VM:"
    echo "  1. Build the VM image using the provided vmConfig."
    echo "  2. Boot the VM."
    echo "  3. Run: scp -r . root@vm:/tmp/pyroduct"
    echo "  4. Run: ssh root@vm 'cd /tmp/pyroduct && ./install.sh --default'"
    echo "  5. Run: ssh root@vm 'pyroduct --version'"
    EOF
    chmod +x $out/bin/test-install-sh
  '';
}
