{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.incipit;
  incipit-pkg = pkgs.callPackage ./default.nix { };

  # Taken from immich
  commonServiceConfig = {
    Type = "simple";

    # Hardening
    CapabilityBoundingSet = "";
    NoNewPrivileges = true;
    PrivateUsers = true;
    PrivateTmp = true;
    PrivateDevices = true;
    PrivateMounts = true;
    ProtectClock = true;
    ProtectControlGroups = true;
    ProtectHome = true;
    ProtectHostname = true;
    ProtectKernelLogs = true;
    ProtectKernelModules = true;
    ProtectKernelTunables = true;
    RestrictAddressFamilies = [
      "AF_INET"
      "AF_INET6"
      "AF_UNIX"
    ];
    RestrictNamespaces = true;
    RestrictRealtime = true;
    RestrictSUIDSGID = true;
  };

  serviceOpts =
    { name, ... }:
    {
      options = {
        port = lib.mkOption {
          type = lib.types.port;
          description = "Port of the service";
        };

        host = lib.mkOption {
          type = lib.types.str;
          description = "Hostname of the service";
          example = "git.example.com";
          default = name;
        };
      };
    };
in
{
  options.services.incipit = {
    enable = lib.mkEnableOption "incipit";
    port = lib.mkOption {
      type = lib.types.port;
      default = 80;
      description = "Port to listen on";
    };

    addr = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      description = "Address to listen on";
    };

    incipit-host = lib.mkOption {
      type = lib.types.str;
      description = "Hostname of the address for the incipit dashboard";
    };

    services = lib.mkOption {
      type = lib.types.attrsOf (lib.types.submodule serviceOpts);
      default = "0.0.0.0";
      description = "Address to listen on";
    };
  };

  config.systemd.services.incipit = lib.mkIf cfg.enable {
    description = "Declarative service manager tailored for home servers";
    after = [ "network.target" ];
    wantedBy = [ "multi-user.target" ];

    serviceConfig = commonServiceConfig // {
      ExecStart = "${incipit-pkg}/bin/incipit";
      StateDirectory = "incipit";
      SyslogIdentifier = "incipit";
      RuntimeDirectory = "incipit";
      User = "root";
      Group = "root";
    };

    environment = {
      "INCIPIT_INPCIPIT_HOST" = cfg.incipit-host;
      "INCIPIT_ADDR" = cfg.addr;
      "INCIPIT_PORT" = "${toString cfg.port}";
      "INCIPIT_SERVICE" =
        let
          mapService = name: service: ''"${name}"={ port=${builtins.toString service.port}, host="${service.host}" }'';
          services = lib.mapAttrsToList mapService cfg.services;
        in
        "{ ${lib.concatStringsSep "," services} }";
    };
  };
}
