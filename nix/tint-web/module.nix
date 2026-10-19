# Serves tint-web for a Tenderly virtual testnet deployment, with a local alto
# bundler for its sponsored operations at /bundler.
{ alto, tint-web }:

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.tint-web;
  # Where tint-web-config writes the /config.json tint-web loads.
  runtimeDir = "/run/tint-web";
in
{
  options.services.tint-web = {
    enable = lib.mkEnableOption "tint-web and its bundler";

    domain = lib.mkOption {
      type = lib.types.str;
      example = "tint.example.com";
      description = "The domain Caddy serves tint-web on, with automatic HTTPS.";
    };

    environmentFile = lib.mkOption {
      type = lib.types.path;
      example = lib.literalExpression ''config.sops.secrets."tint-web".path'';
      description = ''
        The deployment, as `KEY=value` lines, e.g. a sops-nix secret in dotenv
        format. It's read at runtime, so it never enters the Nix store:

        - `RPC_URL`: the virtual testnet's admin RPC URL.
        - `EXPLORER_URL`: the explorer transactions link to, as
          `{url}/tx/{hash}`. A trailing `/transactions` is dropped.
        - `TINT_ADDRESS` and `PAYMASTER_ADDRESS`.

        Every visitor's browser loads these from /config.json, including the
        RPC URL, which it uses directly.
      '';
    };

    bundler = {
      entryPoint = lib.mkOption {
        type = lib.types.str;
        default = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108";
        description = "The ERC-4337 EntryPoint alto bundles for, v0.8 by default.";
      };

      port = lib.mkOption {
        type = lib.types.port;
        default = 4337;
        description = "The local port alto listens on, behind Caddy's /bundler.";
      };

      # Throwaway keys, funded with Tenderly's cheatcodes on start.
      executorPrivateKey = lib.mkOption {
        type = lib.types.str;
        default = "0x4a3a02862ddcb260ed52d40ef03f8e3d78fa3d174b0ef333afdf1ffb4a648cd5";
      };
      utilityPrivateKey = lib.mkOption {
        type = lib.types.str;
        default = "0xdd4b2564c83ff7de602c39ffda1146055dc1814b07c083d7971722384f1f01a6";
      };
    };

    site = lib.mkOption {
      type = lib.types.package;
      default = tint-web;
      defaultText = lib.literalMD "tint-web from this flake";
      description = "The built site Caddy serves.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.tint-web-config = {
      description = "tint-web's /config.json, from its environment file";
      wantedBy = [ "multi-user.target" ];

      script = ''
        ${pkgs.jq}/bin/jq -n \
          --arg rpcUrl "$RPC_URL" \
          --arg explorerUrl "''${EXPLORER_URL%/transactions}" \
          --arg tint "$TINT_ADDRESS" \
          --arg paymaster "$PAYMASTER_ADDRESS" \
          --arg bundlerUrl "https://${cfg.domain}/bundler" \
          '$ARGS.named' > ${runtimeDir}/config.json
      '';

      serviceConfig = {
        Type = "oneshot";
        # Keeps the runtime directory, and so the config, after the script exits.
        RemainAfterExit = true;
        DynamicUser = true;
        EnvironmentFile = cfg.environmentFile;
        RuntimeDirectory = "tint-web";
        RuntimeDirectoryMode = "0755";
        UMask = "0022";
      };
    };

    systemd.services.tint-bundler = {
      description = "alto ERC-4337 bundler for tint-web";
      wantedBy = [ "multi-user.target" ];
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];

      # TintPaymaster breaks the ERC-7562 rules, so alto runs without safe mode.
      script = ''
        for key in ${cfg.bundler.executorPrivateKey} ${cfg.bundler.utilityPrivateKey}; do
          ${pkgs.foundry}/bin/cast rpc tenderly_setBalance \
            "$(${pkgs.foundry}/bin/cast wallet address "$key")" 0x3635C9ADC5DEA00000 \
            --rpc-url "$RPC_URL" > /dev/null
        done
        exec ${alto}/bin/alto --rpc-url "$RPC_URL" \
          --entrypoints ${cfg.bundler.entryPoint} \
          --executor-private-keys ${cfg.bundler.executorPrivateKey} \
          --utility-private-key ${cfg.bundler.utilityPrivateKey} \
          --port ${toString cfg.bundler.port} --safe-mode false
      '';

      serviceConfig = {
        DynamicUser = true;
        EnvironmentFile = cfg.environmentFile;
        Restart = "on-failure";
        RestartSec = 10;
      };
    };

    services.caddy = {
      enable = true;
      virtualHosts.${cfg.domain}.extraConfig = ''
        # Shared wasm memory, which the prover's thread pool needs, requires a
        # cross-origin isolated page.
        header {
          Cross-Origin-Opener-Policy same-origin
          Cross-Origin-Embedder-Policy require-corp
        }

        # Same origin, so the bundler needs no CORS.
        handle_path /bundler* {
          reverse_proxy 127.0.0.1:${toString cfg.bundler.port}
        }

        # Revalidated on every load, so a new deployment takes effect at once.
        handle /config.json {
          header Cache-Control no-cache
          root * ${runtimeDir}
          file_server
        }

        handle {
          root * ${cfg.site}
          try_files {path} /index.html
          file_server {
            precompressed gzip
          }
        }
      '';
    };

    networking.firewall.allowedTCPPorts = [
      80
      443
    ];
  };
}
