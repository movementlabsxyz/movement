# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## [0.3.1](https://github.com/movementlabsxyz/movement/compare/616d714d116e8361583eead2170fe0e7d0f8735c..0.3.1) - 2025-03-13
#### Bug Fixes
- correct add tx to mempool logic (#1106) - ([c56dd90](https://github.com/movementlabsxyz/movement/commit/c56dd9078622e800fe160514613cc1b596e35132)) - Philippe Delrieu
- deduplicate execute_block tracing spans (#1104) - ([ccdfeed](https://github.com/movementlabsxyz/movement/commit/ccdfeedb92fb425057669b7291c4eb86fc0e5d18)) - Mikhail Zabaluev
#### Miscellaneous Chores
- remove unused jupyter notebook (#1105) - ([616d714](https://github.com/movementlabsxyz/movement/commit/616d714d116e8361583eead2170fe0e7d0f8735c)) - Richard Melkonian

- - -

## [0.3.0](https://github.com/movementlabsxyz/movement/compare/a156114149ef6e7329fed7cd22d1420d3df2be94..0.3.0) - 2025-03-12
#### Features
- partial fix seq-num (#1100) - ([a156114](https://github.com/movementlabsxyz/movement/commit/a156114149ef6e7329fed7cd22d1420d3df2be94)) - Richard Melkonian

- - -

## [0.2.0](https://github.com/movementlabsxyz/movement/compare/4d86f21c69e1feac44a6e2897eeb68573b5929b5..0.2.0) - 2025-03-11
#### Features
- timing instrumentation for executed blocks and transactions (#1103) - ([579d353](https://github.com/movementlabsxyz/movement/commit/579d3538fb3e60e33008f5d1c801d43ada072a18)) - Mikhail Zabaluev
#### Miscellaneous Chores
- reverted aptos core in cargo lock - ([fa34787](https://github.com/movementlabsxyz/movement/commit/fa34787284b4766b690fd4d0c0b5e48f311eb031)) - Nicholas McMillen
- revert aptose core version bump - ([4d86f21](https://github.com/movementlabsxyz/movement/commit/4d86f21c69e1feac44a6e2897eeb68573b5929b5)) - Nicholas McMillen

- - -

## [0.1.2](https://github.com/movementlabsxyz/movement/compare/4908c91905d3732f8907d0bcf653e8d639f7491f..0.1.2) - 2025-03-11
#### Bug Fixes
- build on tag (#1097) - ([4908c91](https://github.com/movementlabsxyz/movement/commit/4908c91905d3732f8907d0bcf653e8d639f7491f)) - radupopa369

- - -

## [0.1.1](https://github.com/movementlabsxyz/movement/compare/458bf73817a0e53432492845817e8bf51710be75..0.1.1) - 2025-03-10
#### Miscellaneous Chores
- update aptos core rev (#1095) - ([458bf73](https://github.com/movementlabsxyz/movement/commit/458bf73817a0e53432492845817e8bf51710be75)) - Richard Melkonian

- - -

## [0.1.0](https://github.com/movementlabsxyz/movement/compare/8bd5218892a8e493a25309ef2e012463bc3c3543..0.1.0) - 2025-03-09
#### Bug Fixes
- **(docker)** celestia-light-node fixes (#1078) - ([9dbb761](https://github.com/movementlabsxyz/movement/commit/9dbb76125a4d4759ab4da19db83ef19ceb69cd90)) - Mikhail Zabaluev
- trigger build and check when push to main (#1090) - ([083de2a](https://github.com/movementlabsxyz/movement/commit/083de2a2f73851d1ad32d401b360281d3ef558b0)) - radupopa369
- add back labeled builds (#1072) - ([164420e](https://github.com/movementlabsxyz/movement/commit/164420e53d6992a74b610a0330cee1eeb3f7f9d4)) - radupopa369
- HTTP2 Connection Should be Used for DA Tool (#1085) - ([92a4cdb](https://github.com/movementlabsxyz/movement/commit/92a4cdb6781d8136966db9dfe8ec95e28204bc54)) - Liam Monninger
- Update executor block validator signer to use LoadedSigner, add test key admin command (#1063) - ([e407f64](https://github.com/movementlabsxyz/movement/commit/e407f641c854457e03daffc674c72eca19914e89)) - Philippe Delrieu
- Correct DA client http2 connection (#1079) - ([8d89aac](https://github.com/movementlabsxyz/movement/commit/8d89aac3e3d34736c3963eb6ac4edb1907ebfa22)) - Philippe Delrieu
- Upgrade script to use usecs after timestamp upgrading timestamp logic (#1075) - ([67d303a](https://github.com/movementlabsxyz/movement/commit/67d303af80cbd1dce41bd7710352caeb2341ac2d)) - Liam Monninger
- Celestia mainnet config (#1038) - ([338d56b](https://github.com/movementlabsxyz/movement/commit/338d56b724fabc4b54e6bef1c74a95f0fb4528c6)) - Mikhail Zabaluev
- Gas Upgrades Beta Fixes pt. 2 (#1070) - ([fcfafac](https://github.com/movementlabsxyz/movement/commit/fcfafac3b03ec01f0afacf07cde84d84100cc6e4)) - Liam Monninger
- Update light client protocol and heartbeat. (#1064) - ([69b2e6c](https://github.com/movementlabsxyz/movement/commit/69b2e6cab8247f49fc231a6c1a0ea1fa665a0161)) - Philippe Delrieu
- Add retry on DA connection (#1054) - ([ffb4633](https://github.com/movementlabsxyz/movement/commit/ffb463324321b06e21fc08cbbfd75304f7c4b3fe)) - Philippe Delrieu
- Gas Upgrades and Beta Fixes (#1055) - ([fd54c29](https://github.com/movementlabsxyz/movement/commit/fd54c29fffb81c38c2321f4393a9ffbf23d00c77)) - Liam Monninger
- Change Hashicorp Vault delimiter to a - (#1061) - ([d7fa03a](https://github.com/movementlabsxyz/movement/commit/d7fa03a30922b38b888bee1799615dab3e55d20f)) - Philippe Delrieu
- add testnet in key name environement (#1059) - ([27b93fe](https://github.com/movementlabsxyz/movement/commit/27b93fe3d3752c016213c6aa14b974611818860d)) - Philippe Delrieu
- MOVEMENT_SYNC is not longer mandatory  (#1053) - ([b877cb1](https://github.com/movementlabsxyz/movement/commit/b877cb11465a1c57c06d603d6f02eac60280b8f8)) - Philippe Delrieu
#### Continuous Integration
- fix unit tests (#1069) - ([750d594](https://github.com/movementlabsxyz/movement/commit/750d5947867668321dfc8792b4a75c6071639899)) - Mikhail Zabaluev
#### Documentation
- update READMEs (#685) - ([c5f04e7](https://github.com/movementlabsxyz/movement/commit/c5f04e7008a3aa7bfaaefe1de308fd31d34fec62)) - Andreas Penzkofer
- added information on deploying the indexer (#1037) - ([8bd5218](https://github.com/movementlabsxyz/movement/commit/8bd5218892a8e493a25309ef2e012463bc3c3543)) - radupopa369
#### Features
- Upgrade Framework Script w/ Burn (#1084) - ([f1924cc](https://github.com/movementlabsxyz/movement/commit/f1924ccf5f7d161e9b24304fa75867afea8a8a68)) - Liam Monninger
- Add ans processor (#1081) - ([bbb8be6](https://github.com/movementlabsxyz/movement/commit/bbb8be6c665d7602862f6b26997ed8e72f8b0920)) - Philippe Delrieu
- add curl in the final container - ([07da9d4](https://github.com/movementlabsxyz/movement/commit/07da9d474992477125b6917ac2b0bd898de33a44)) - Radu Popa
- Backup / restore follower DB (#1051) - ([1d435a9](https://github.com/movementlabsxyz/movement/commit/1d435a9118afce0c271e453b441b33ca61fed0a6)) - Philippe Delrieu
- movement-full-node-v0.0.1-beta (#1048) - ([63e9323](https://github.com/movementlabsxyz/movement/commit/63e9323ef9d7bb7cba32b59d6b3a5d2eea6ae8e9)) - Liam Monninger
#### Miscellaneous Chores
- add back in missing submodule and mrb_cache (#1094) - ([c647978](https://github.com/movementlabsxyz/movement/commit/c64797837f8430bcb9aae3c88a1023d4198a0533)) - Richard Melkonian

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).