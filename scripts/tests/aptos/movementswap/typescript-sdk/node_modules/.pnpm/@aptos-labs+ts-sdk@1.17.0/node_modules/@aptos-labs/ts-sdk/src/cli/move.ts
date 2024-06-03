import { spawn } from "child_process";
import { platform } from "os";

import { AccountAddress } from "../core";
import { Network } from "../utils";

export class Move {
  /**
   * Function to initialize current directory for Aptos
   *
   * Configuration will be pushed into .aptos/config.yaml
   * @param args.network optional Netowrk type argument to use for default settings, default is local
   * @param args.profile optional Profile to use from the config file, default is 'default'
   * This will be used to override associated settings such as the REST URL, the Faucet URL, and the private key arguments.
   *
   * @returns
   */
  async init(args: { network?: Network; profile?: string }): Promise<boolean> {
    const { network, profile } = args;
    const cliArgs = ["aptos", "init", `--network=${network ?? "local"}`, `--profile=${profile ?? "default"}`];

    return this.runCommand(cliArgs);
  }

  /**
   * Function to compile a package
   *
   * @param args.packageDirectoryPath Path to a move package (the folder with a Move.toml file)
   * @param args.namedAddresses  Named addresses for the move binary
   * @example
   * {
   *  alice:0x1234, bob:0x5678
   * }
   *
   * @returns
   */
  async compile(args: {
    packageDirectoryPath: string;
    namedAddresses: Record<string, AccountAddress>;
  }): Promise<boolean> {
    const { packageDirectoryPath, namedAddresses } = args;
    const cliArgs = ["aptos", "move", "compile", "--package-dir", packageDirectoryPath, "--skip-fetch-latest-git-deps"];

    const addressesMap = this.parseNamedAddresses(namedAddresses);

    cliArgs.push(...this.prepareNamedAddresses(addressesMap));

    return this.runCommand(cliArgs);
  }

  /**
   * Function to run Move unit tests for a package
   *
   * @param args.packageDirectoryPath Path to a move package (the folder with a Move.toml file)
   * @param args.namedAddresses  Named addresses for the move binary
   * @example
   * {
   *  alice:0x1234, bob:0x5678
   * }
   *
   * @returns
   */
  async test(args: { packageDirectoryPath: string; namedAddresses: Record<string, AccountAddress> }): Promise<boolean> {
    const { packageDirectoryPath, namedAddresses } = args;
    const cliArgs = ["aptos", "move", "test", "--package-dir", packageDirectoryPath, "--skip-fetch-latest-git-deps"];

    const addressesMap = this.parseNamedAddresses(namedAddresses);

    cliArgs.push(...this.prepareNamedAddresses(addressesMap));

    return this.runCommand(cliArgs);
  }

  /**
   * Function to publishe the modules in a Move package to the Aptos blockchain
   *
   * @param args.packageDirectoryPath Path to a move package (the folder with a Move.toml file)
   * @param args.namedAddresses  Named addresses for the move binary
   * @example
   * {
   *  alice:0x1234, bob:0x5678
   * }
   * @param args.profile optional Profile to use from the config file.
   *
   * @returns
   */
  async publish(args: {
    packageDirectoryPath: string;
    namedAddresses: Record<string, AccountAddress>;
    profile?: string;
  }): Promise<boolean> {
    const { packageDirectoryPath, namedAddresses, profile } = args;
    const cliArgs = [
      "aptos",
      "move",
      "publish",
      "--package-dir",
      packageDirectoryPath,
      "--skip-fetch-latest-git-deps",
      `--profile=${profile ?? "default"}`,
    ];

    const addressesMap = this.parseNamedAddresses(namedAddresses);

    cliArgs.push(...this.prepareNamedAddresses(addressesMap));

    return this.runCommand(cliArgs);
  }

  /**
   * Run a move command
   *
   * @param args
   * @returns
   */
  // eslint-disable-next-line class-methods-use-this
  private async runCommand(args: Array<string>): Promise<boolean> {
    return new Promise((resolve, reject) => {
      const currentPlatform = platform();
      let childProcess;

      // Check if current OS is windows
      if (currentPlatform === "win32") {
        childProcess = spawn("npx", args, { shell: true });
      } else {
        childProcess = spawn("npx", args);
      }

      childProcess.stdout.pipe(process.stdout);
      childProcess.stderr.pipe(process.stderr);
      process.stdin.pipe(childProcess.stdin);

      childProcess.on("close", (code) => {
        if (code === 0) {
          resolve(true); // Resolve with true if the child process exits successfully
        } else {
          reject(new Error(`Child process exited with code ${code}`)); // Reject with an error if the child process exits with an error code
        }
      });
    });
  }

  /**
   * Convert named addresses from a Map into an array seperated by a comma
   *
   * @example
   * input: {'alice' => '0x123', 'bob' => '0x456'}
   * output: "alice=0x123,bob=0x456"
   *
   * @param namedAddresses
   * @returns An array of names addresses seperated by a comma
   */
  // eslint-disable-next-line class-methods-use-this
  private prepareNamedAddresses(namedAddresses: Map<string, AccountAddress>): Array<string> {
    const totalNames = namedAddresses.size;
    const newArgs: Array<string> = [];

    if (totalNames === 0) {
      return newArgs;
    }

    newArgs.push("--named-addresses");

    let idx = 0;
    namedAddresses.forEach((value, key) => {
      idx += 1;
      let toAppend = `${key}=${value.toString()}`;
      if (idx < totalNames - 1) {
        toAppend += ",";
      }
      newArgs.push(toAppend);
    });
    return newArgs;
  }

  /**
   * Parse named addresses from a Record type into a Map
   *
   * @param namedAddresses
   * @returns Map<name,address>
   */
  // eslint-disable-next-line class-methods-use-this
  private parseNamedAddresses(namedAddresses: Record<string, AccountAddress>): Map<string, AccountAddress> {
    const addressesMap = new Map();

    Object.keys(namedAddresses).forEach((key) => {
      const address = namedAddresses[key];
      addressesMap.set(key, address);
    });

    return addressesMap;
  }
}
