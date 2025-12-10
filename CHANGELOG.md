# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-12-10

### ♻️ Refactoring

- Code quality improvements ([3a8b53b](https://github.com/nebucloud/nebucloud-xds/commit/3a8b53b1f784a34a506bd62db3123beb6052fbef))

- Share SotwHandler and add streaming helpers ([528204d](https://github.com/nebucloud/nebucloud-xds/commit/528204d8bb1b826513090acaf0e2104d2b0592a5))

- Rename data-plane-api to proto ([ae38ea1](https://github.com/nebucloud/nebucloud-xds/commit/ae38ea16eceec4ae00763f71746824272618ab91))

### ⚙️ Miscellaneous

- **proto:** Update proto submodules to latest versions  ([887358d](https://github.com/nebucloud/nebucloud-xds/commit/887358deeab141999dd5965ea08be7f72812f82d))
- Update packages ([5ae8dba](https://github.com/nebucloud/nebucloud-xds/commit/5ae8dbaac4c91b67ab56d90356a43cf9f538cb6c))

- Update Cargo.lock ([a61a650](https://github.com/nebucloud/nebucloud-xds/commit/a61a6509bdec4940d99da70e7e7c161d4ca9aa2d))

- Configure GitHub Actions CI pipeline  ([17eb36f](https://github.com/nebucloud/nebucloud-xds/commit/17eb36f64b36d1311588d1d0066112792dd8d19d))

- Configure Dependabot for dependency updates ([e592b39](https://github.com/nebucloud/nebucloud-xds/commit/e592b392947bcee37ff6690b6bb73977d5134703))

- Add proto-sync automation workflow  ([c9c42a6](https://github.com/nebucloud/nebucloud-xds/commit/c9c42a6546346d414c748b68abc37ed036382ff0))

- Set up Changesets and release workflow  ([357a676](https://github.com/nebucloud/nebucloud-xds/commit/357a67614f7fa2b58253fd420efb57a288675240))

- Remove legacy rust-control-plane and test-xds directories ([7709927](https://github.com/nebucloud/nebucloud-xds/commit/770992750ebbe274b5edc5e50f2157092da3bfa9))

- Add dual MIT/Apache-2.0 licensing ([c07a2c0](https://github.com/nebucloud/nebucloud-xds/commit/c07a2c0feffb657b03d5ddaf1fbd659f81b147c8))

- Align all crates to version 0.1.0 ([d8a9aeb](https://github.com/nebucloud/nebucloud-xds/commit/d8a9aebaabd060c8a7cfd07f60590e349512152e))

### ✨ Features

- **cache:** Add comprehensive unit tests for cache module  ([796ad03](https://github.com/nebucloud/nebucloud-xds/commit/796ad036ee97be4bf8ae9b99ff8f8cdee4ab0cb5))
- **examples:** Add custom-control-plane example for external consumers ([5a52102](https://github.com/nebucloud/nebucloud-xds/commit/5a52102e07dbf0b5251b1bdc5b8c9f801036ed5c))
- **xds-core:** Add SharedResourceRegistry for thread-safe type management  ([d5ecd6c](https://github.com/nebucloud/nebucloud-xds/commit/d5ecd6c4ca969d1a4b0257cf9ef328a544d849ce))
- **xds-server:** Production Readiness Features (M4)  ([ed46c40](https://github.com/nebucloud/nebucloud-xds/commit/ed46c40b408f2257d33db08c46ae9ab1fb7dc360))
- **xds-server:** Integrate data-plane-api generated types ([79f72fb](https://github.com/nebucloud/nebucloud-xds/commit/79f72fb687dcd1261aba17baf7a0040b76d087f7))
- Init cargo workspace ([2788737](https://github.com/nebucloud/nebucloud-xds/commit/278873767be33539e3f158c5d1542496a718be8b))

- Add data-plane-api submodule ([f599c4a](https://github.com/nebucloud/nebucloud-xds/commit/f599c4a927ca4a22bf9944096647c287946f6b07))

- Add xds submodule ([76deb59](https://github.com/nebucloud/nebucloud-xds/commit/76deb5972f79ccebfb67e238ebd7735d0da55943))

- Add googleapis submodule ([80ac943](https://github.com/nebucloud/nebucloud-xds/commit/80ac943df4cf025bbb5d09fb68423144ca9462b1))

- Add protoc-gen-validate submodule ([7726155](https://github.com/nebucloud/nebucloud-xds/commit/7726155b30fd71403f29a68a5a1ca1524ee86e63))

- Add metrics protos ([2c907d1](https://github.com/nebucloud/nebucloud-xds/commit/2c907d1b9d0272db056216d3a3785c7a8743fc8f))

- Add tokio and prost ([bddaec6](https://github.com/nebucloud/nebucloud-xds/commit/bddaec6a9d7f3367e5af5f43932995f1333d0bb2))

- Setup build for data plane api ([7450fca](https://github.com/nebucloud/nebucloud-xds/commit/7450fcaf79dd59d28f84b31c324834adf125bc99))

- Setup rust-control-plane ([b3e78e0](https://github.com/nebucloud/nebucloud-xds/commit/b3e78e059a4223cd5b1ecd53daf73f2646507af1))

- Add unit test ([cfd03fc](https://github.com/nebucloud/nebucloud-xds/commit/cfd03fc00c728604f88004bc03a55a8fe31c4d20))

- Update packages ([a145ddb](https://github.com/nebucloud/nebucloud-xds/commit/a145ddb17863fa7c8e38e193dd9aadda6425386f))

- Add config.yaml and delta/ads configs for envoy ([dd0981a](https://github.com/nebucloud/nebucloud-xds/commit/dd0981a884a11edb0f7cb94ea20e1e4fabd3ba3f))

- Create 5-crate workspace structure with foundation  ([77a009a](https://github.com/nebucloud/nebucloud-xds/commit/77a009ace1d8c1b8547b6e0631a00a42023fdfc3))

- M3 Protocol Handlers - Complete xDS Protocol Implementation  ([297ffae](https://github.com/nebucloud/nebucloud-xds/commit/297ffaea634b440eb8fec9c59730953c36b88771))

### 🐛 Bug Fixes

- **xds-server:** Propagate encoding errors and validate node in first request ([a9efd1a](https://github.com/nebucloud/nebucloud-xds/commit/a9efd1a2a4e7d0829ad91dfb108506466f6bf0fe))
- Handle errors and stream termination in handle_stream ([a3f2518](https://github.com/nebucloud/nebucloud-xds/commit/a3f2518ac098f64c5aee71e00d79900ec31dc0df))

- Fix error handling in spawn ([cf6146f](https://github.com/nebucloud/nebucloud-xds/commit/cf6146f4dd06dbecdfdd5d853512098591fb67c1))

### 📚 Documentation

- Update MILESTONES.md - mark #3 and #4 complete ([f9870bb](https://github.com/nebucloud/nebucloud-xds/commit/f9870bb404140400a0d85fe84a2239b58c709c41))

- Update MILESTONES.md - Issue #2 complete (75% M1) ([7f2960b](https://github.com/nebucloud/nebucloud-xds/commit/7f2960bc932c2a541660d7deda924ac617f9be3d))

- Update MILESTONES.md - Issue #5 complete (87.5% M1) ([1275e8c](https://github.com/nebucloud/nebucloud-xds/commit/1275e8ca176d248b6a68955b1d37d2a5519c11da))

- M1 Foundation complete! (100%) ([e99a2d1](https://github.com/nebucloud/nebucloud-xds/commit/e99a2d11e7be080d3ff58c8083ed877ad72ce97c))

- Mark M2 Core Implementation as 100% complete ([6d5e426](https://github.com/nebucloud/nebucloud-xds/commit/6d5e426c6e6ebced202eeb6d57bcbc185a6dab80))

- Add custom control plane section to README ([2003735](https://github.com/nebucloud/nebucloud-xds/commit/200373567ebe0009cee93237905935e2d724f489))

### 🧪 Testing

- Add test case for removing endpoints from a cluster ([25000da](https://github.com/nebucloud/nebucloud-xds/commit/25000daf279d4829f1bb311cf05ce2bacbd21261))

- Add test case for hiding a cluster ([8318aa9](https://github.com/nebucloud/nebucloud-xds/commit/8318aa9735de6c9340658ef8b533ab136ac792bb))

- Add test case 7 to update snapshot with a different endpoint ([9b901bd](https://github.com/nebucloud/nebucloud-xds/commit/9b901bda9bdf75a70671e5eeca8e2546681aefd1))

- Add stress test to ensure that the xds server can handle ([12ffecf](https://github.com/nebucloud/nebucloud-xds/commit/12ffecf2cf3bf109ddff84b68a104357dcf373fe))
---
*Generated by [git-cliff](https://git-cliff.org)*
