# CHANGELOG

<!--- next entry here -->

## 0.36.0
2025-03-01

### Features

- Add levels to captureworker logs (8345a0d7d0393d1ee47c5629da809d40c85d9ec7)
- Update to Rust 2024 edition (9357e82b259c062a19ab3d801e4e80d4b1fe0701)

### Fixes

- Prepare for 2024 edition (0b25914b8b2bb0645f356a0f008a0a8b7265ca00)
- Update base docker images (24cc17e3e9b17c2a29327d7866f56d5201543eab)
- Update to Rust 1.85.0 (9fe6beb94139ef77b9c39a38e2daf247e98af1f2)

## 0.35.2
2025-01-15

### Fixes

- update rust to 1.84.0 (74d6778f49d4ae3537d39f54ded88f41ba3947b8)
- appease clippy (5c9424698f70903f5eabb2cb17493942a62e1bd4)

## 0.35.1
2025-01-12

### Fixes

- switch from cargo make to gnu make (8bcb089c244aed8f54bd2bb43a509274434cfbc6)

## 0.35.0
2025-01-07

### Features

- switch to mold for development (8c328ef81a383b91e5c019415c07bacea788706c)

## 0.34.0
2025-01-02

### Features

- Create metrics to track rtp packets lost by CaptureWorker (3a54d6cdaab754b6845e451d4de5443040f86555)
- Switch to pgautoupgrade (75db711f7dda347c2d680205fe28a550c2974005)

### Fixes

- fix typo in tag attribute name (a56cbdbcc12cec14339fe55ed417d62df4ff53e1)
- appease clippy (0977d2f825fad43ca82506fad837bb590abeb433)

## 0.33.0
2024-12-20

### Features

- Add orderable "ALL" CameraGroup (6c590bc615fa4d133ffe3eca5407688f42972f9d)
- Add deployment to ci (df21f0c82df8a93660e1a11a10bd4fcaad4c3d26)

### Fixes

- appease clippy (5ee59dc8d2505f9aa473a3ff5d2b197336bc0949)
- "fix" db tests, remove nullable db (5a0da0ea7ad9a35c2e0d7ed1559041d517dd2273)
- improve error logging (ae1e5f03dbda5e2d4138d2e7ee13094f6ae93d25)
- switch to uuid for Cameras and CameraGroups (e6bacf626b78880e5d4d220e8c0ad8c333508554)
- Remove unused subscription fields (44f1902d3ad735fb4e03768cc626878c987b12c0)
- camera creation api (fb0e80c3cba6bf541656146b77c431f8b3ab13c7)
- CameraGroup editing (1f3c542de03bee23bfa25e52ed6c9bda6f1c429e)
- camera service filtering (c9f022c2aeb4d7ba81c340baa58a244fe58c56f1)
- gitlab-ci formatting (44de047d9de10888d00e77c838d0fe6e884b12ef)
- remove deploy job (073a36986f99f0b0480867d0507725c6a5a0b9d1)
- CameraGroupDetailComponent formatting (e7f9c12ce3bba268dc18c0031ff677b91f5505c6)

## 0.32.2
2024-10-02

### Fixes

- Update cargo deps (8cc4aa94d1e1080604bb528cc47014402caa7d1d)

## 0.32.1
2024-09-05

### Fixes

- Update docker base images (cff21e6ba54222039d4b5dbc4e1737eff6e42434)

## 0.32.0
2024-08-09

### Features

- Add metrics route (6da59898025f2d35e623c0acf3b470b429b4df53)
- Add gauge for webrtc_sessions (3a730e0ac0a7839598ca61d2a9ee3eec1c573e1e)

### Fixes

- upgrade to Axum 0.7 (06757bbf049c6342f8b6587ff4672eb418a7e76d)
- kill CaptureActor if child process dies (8d069d5cd7ec5cdb99ab27cd8da2bef6f71a80ef)

## 0.31.1
2024-07-22

### Fixes

- Fix switching groups when they contain disabled camera (24a6a82cb193131ee5a28c91633881c52d6bec46)
- Fix referencing mediaStream (04f1b8afdf68c518e569f71695e14d764616a9de)

## 0.31.0
2024-07-20

### Features

- Allow webrtc udp buffer sizes to be set with an env variable (9029041f950b231cefd5e153a9f07737345e0796)

## 0.30.1
2024-07-14

### Fixes

- remove bwe (3d4177c6413c1142e08a8bded909f9eaa6fe272a)

## 0.30.0
2024-07-02

### Features

- Add camera name and id to capture actor logs (8f53cc262ea096743ad4a9ec2491237221a4dfd1)

### Fixes

- Capture verbose ffmpeg logs from captureworker (96ac3d77a2f3647c1c9bd4a5baa885acf247c150)
- **web:** switch from cloudflare to desec.io (8b88d2c8ac1e94f68b72da44dd41e55787a57643)
- Add webrtc ips envvar to prod docker-compose (248675323050f9a53063d23224187e1fe7be370c)
- Enable log timestamps (d47430d8b864245dcfa4e157db1401bfa9ab18b8)
- set capture buffer size to 4 MiB (755da6646cb96a67ff340086438dc972440a089f)
- 2021 edition (ed98cc7b5e5a0c80d495c18ccbd2145207fa8c03)
- Remove extra event handlers (7038bc4f44b1eae5de06e38b18a04377a31b961f)
- Make camera supervisor log debug (1a076a7fb98d6826681519994c5237586ac962b4)

## 0.29.0
2024-06-14

### Features

- Add log level to exserial log messages (83b70f52c9742c85a932091ace706c0640475e4c)

### Fixes

- Use correct web asset path (03f951b90b8d3ef7ad0f3ce0db68c27ae1e3e2b0)

## 0.28.1
2024-06-13

### Fixes

- parse candidate ips first (d0d79cff7e5aee2157e22f7aef35ea6372ae514c)
- Bump Rust version (d3ca1d3cfb50230d8deaa02a1a5f08b0f2eb5ed9)
- Bump npm versions (45f20804ab6c42e662756d13110b1b1897a9e6ca)
- update to Angular 18 (1c262ca6a75fc54f060abbcfc676226e1234fb55)

## 0.28.0
2024-05-16

### Features

- Add ptz step size (71f4bd44f93613f571774de22424da8b5e3218c4)
- Enable alternative ports and hostname candidates (2c95da8f44edbcb09a2100f036def2beb5ae3663)

### Fixes

- update str0m (704932febf1e179677ed80e5e8b5712c4fd052a0)

## 0.27.0
2024-05-13

### Features

- WIP webrtc video streaming (3ceff4c63f091278e10c61cd1f95d16922909bd4)
- Add relative ptz move route (f58596c6557484867d9c57f6f372644c571bf3ab)
- CaptureSupervisor: restart a single CaptureActor if stopped (672efcc2bbef4b2dc730f1411e59c26baecfef1a)
- camera-status-overlay: add camera name (e3a080ee74162ac42d68cc21387148cb07e48a2f)
- WIP initial broken video streaming with str0m (fb873a73994b56d2471b7a9d925f16f0150f5e0c)
- switch to cloudflare validation for caddy certs (d0b00589be66123150d523e9dc6dd08205fafffe)
- **ci:** Remove deploy step (b9fb6e1ac00845434e1bb41700e4d93ad147951a)

### Fixes

- fix playbackworker warnings (0810ced3c9e23323c75028a52add6ea249becaee)
- remove unused function (4b778926a779ce9bd169fdac0b29266e499e424f)
- extract jpeg encoder function (233ae86286397364a47ca3854057b565ee288c84)
- remove mpack_frame.c because we don't use it anymore (173b02f342b67f98e9266454bab3be1d250c87d9)
- Used calculated scaled heights instead of hardcoded values (c90cad247a9f7e25aa27eb6f579e40b54d11e3fa)
- WIP (fffd594cc75c4a364b83af689e7c963ba3a5c4b9)
- Update Rust to 1.66.1 and appease Clippy (fbcc9a0b1d8d0d05f3fefe5a42e9e683668e3aa6)
- Add timestamps and durations to video packets (bf07c84fb1e64da5288dac6f17766ebf9f24b19f)
- appease clippy (b7973707ad3ba10812028bf2bc4d318e94fb7b17)
- updateState when websocket connects (41d482af29baf63cdfeb56be822094a95c0f2437)
- axum refactor (693eadce8ad10d1efe91337a6b6abeced3cc61ff)
- Implement new deletion actor (401625e9972bda71609065a074f5a9296699a702)
- Remove actix_web (1999fed6d37e4794b4877412b941950b5443f492)
- fix warnings (c571ff9a299688f0ed6567c51979e053c1c8cabb)
- appease clippy (3d52ce4309eb1fe9f3210218d257b8dc27958938)
- fix release build args (2ca1880522650f79cd174994fdaa79f34104b91c)
- Add webrtc_ips env var (d1f28e983eb0014914b67f8c75397440111e4b26)
- use env log levels (810d6fa0c4d9ac80d1da42a5ecc2c2b10baba3ac)
- Fix db tests (0f9f751ca782219f1986a1f999ddd8ea90226af1)
- fix formatting (d79ec157f5f6cbd31d474870079d19b39bde1fb8)
- Make sure api response are camelCase (d8a99610ea547902cfa644b69996366c9acf6025)
- wait for child exit in CaptureActor (152f8a69ed9b1a9900e7ecf63bf027c9975d01b4)
- remove old noisy logs from camera-panel.service (00506a3007cbaebd5b4a004ff8af4aa9b5b21e28)
- Add route for manifest.webmanifest (b93b8860cbdf5a616d336a935958662447d5ace4)
- **web:** Fix the webmanifest route (08889bff036ab4db7e1e6d6882a9befe276193da)
- appease clippy (10e63291635b1271fcbb5c959fa48039504d2bea)
- Fix static asset loading (9a255df92ee0152c2c095ef9a54b8926db2be33e)
- Set fixed name for webmanifest (3c56b8988eda510457ffc1dfce4eebb7aafc5a74)
- fix css formatting (e85e33811314b3f7a57ef869ccde1cf27df36206)
- remove slim build (a85777fba4dd3f96f9f816b93371515492c3f4a3)

## 0.26.5
2022-11-11

### Fixes

- add cuda to deploy (7a70aa647dac6a3f69664214bcc58d78d25c5668)

## 0.26.4
2022-11-11

### Fixes

- Add cuda to deploy (87fc8fa1ae8d7f5c172f31b724b97d7506ef0a1d)

## 0.26.3
2022-11-11

### Fixes

- Add pg password env to deployment (f2c0d5dc402603f77807fc8a5fc4af33341bac03)

## 0.26.2
2022-11-11

### Fixes

- ci/cd deploy (d3c3f19724e8659caf15c9ca5913bb77e7844d55)

## 0.26.1
2022-11-11

### Fixes

- deployment (f72d1e80770d87e2e366c57e07048a0615e5f5d8)

## 0.26.0
2022-11-05

### Features

- Implement initial CameraGroup ui (9f44062683e22a8bbc4526bf33d2008c033893d7)

## 0.25.2
2022-11-01

### Fixes

- Dockerfile: Pin base images with SHA256 (c92f65e273a5817ea665506581f7da8ec03fd5c1)

## 0.25.1
2022-10-26

### Fixes

- CameraPanel timeouts (af4d84ca024bc004dd12887ff59f1578d37026d9)

## 0.25.0
2022-10-21

### Breaking changes

#### Rename camera groups to storage groups (e6916bc397f5bbe5fbb97fbdeaecaf5e66ca0eb9)

remove camera group endpoints and replace with
storage groups

### Features

- Implement CameraGroup api (cc96b98cbf41759fc1112472d597acd7ef861703)
- Implement CameraGroup business logic (409b5777ef9d337fb3add323ffab624c280f32d6)

### Fixes

- Quiet analysis logs (3cd221a6c57bdc28b27684e89f17ee9750e2bd67)
- give event clips a nicer filename (e2d00face6f5f1adc42e4e8648fe900cfb2e9292)
- Upgrade to Angular 14 (ea76a1a41515bdd08c66a13206c66b04e2cd963f)
- fix CameraGroup integration tests (31e9ecf06d037636df0e4cc4b9cedaf6bfb0db0d)
- pin pathspec version to fix dvc (e2043da8f7721d8590b2a3d99606d9fba117db24)

## 0.24.3
2022-08-01

### Fixes

- remove cuda config from prod (888a26a8c5c507d38237a5411461df3d0580fcf7)

## 0.24.2
2022-04-21

### Fixes

- Update Angular to 13.3.3 (82bcf43e0cd6a24a8b04ee40bf11348328361988)
- web audit fixes (be50c35f58bb3c03672d6c899e617cc397649a0b)
- update web deps (3fd274f55a51dfc718a85573d9794f26dcf25c93)

## 0.24.1
2022-04-08

### Fixes

- fix top menu style (a6ed9086a3411c1acd64dd3d36996849acf750bc)
- Allow camera panel to be less than viewport height (9b83c7c7ec4e98cb1b0b75ce84d5b1dd82596f58)
- Allow touch events to toggle camera overlay (6de673d0543494c928620f3cd7571035f22528f9)
- CameraPanel: remove unused variables, methods (52980aab56143628d94dd24b938a4d5359a29eeb)

## 0.24.0
2022-04-07

### Features

- Add token delete route (456a4f19cff7f780682a56e5e81721489acfdb60)
- Add personal access token list (eb4decd98c6888f908fc538daaab668b01b7063e)

### Fixes

- use 2021 (13975c8ceb7aac1d3e3ec4b26197e4ffba2c0741)
- fix token-list formatting (96eea502c17c345e9e5f2320d6c10f210d5b6913)
- use forwarded ip from header for logs (2a60872fa99eec407ac474ed469fff55e1938700)
- fix css property name (5b57f7eac691908928d02b2722475f2c99b154dd)
- change button/header styles (bf82df0184496bba45276278730d0b9a6cbc2d8c)

## 0.23.0
2022-03-29

### Features

- Add personal access token routes (950488bcd0b321e355039277a9709b6b27ff7142)

## 0.22.9
2022-03-26

### Fixes

- fix /v1/user/me route (a77bcec3e02cb7d4749992817c399b11cc497272)

## 0.22.8
2022-03-26

### Fixes

- implement server-side sessions (e01761dd7c7763bee417f79ed821affc2f7be5d1)

## 0.22.7
2022-03-21

### Fixes

- remove telegram actor (f952b7f370298aa0f595ff79f9f526adc318d925)
- Update tokio to v1 (579f5a8079cbfb8de6b9688e54cac7e0ae9dcd63)
- update Cargo.lock (58c7f803cb6fd5b14839ccc34a78fe5911fe6ab4)

## 0.22.6
2022-03-18

### Fixes

- Update to Rust 1.58.0 (15ed260d817377fef9b35959ed02bbcd25cdd426)
- Update to actix-web 4 (4817c7cd8574f57d19cedacc1162899093e5cabf)

## 0.22.5
2022-02-22

### Fixes

- Add thiserror and eliminate panics (230d3a7645f54b6f9e80510b4195a0f76b17f1a3)
- Upgrade thiserror to appease clippy (28254d933817c11cd2bc04baee7ca2572d2f3516)

## 0.22.4
2022-02-17

### Fixes

- Remove keep file. (ba0eec6db577013db25413be680e2e396ba2b272)
- Upgrade to Angular 13 (90d709f2a71fb8389ef3d2ef1f03cdd27184edb5)
- update packages. (407ffecaf4d63036d4fb49c3b76012da59b04c5a)
- Update node to v16 (e770536af8ff398417713e9851496cd5543eb8db)
- Remove empty lines in continuation (1c9977b6751ac0c4798ef046005ca00f8dd1aabd)
- remove unused npm dependency (66851b356312c9da09bfda0c31bf124e8342700b)
- Update js-joda (60885735fa75b78dfa9f33f8f4e4ca3130214b96)

## 0.22.3
2022-01-30

### Fixes

- use new syntax for nvidia container toolkit (b80cdae2dfea91282f2e6f5796c44b26d9006acc)
- Add 'npm install' so initial build works (c0cd38ce52d7ede2980b0ec2a0a9c0125cef6d22)
- Check if observation snapshots exist before creation (111bbb7132a0e41f844e263b02cf451519717467)
- appease clippy (8c5ebae0e010575b1469d9797946865e52d902fd)

## 0.22.2
2022-01-13

### Fixes

- remove unnecessary quotes around vaapi (f3fe481125cedd80d661e3318e99951fbfce4b39)
- Install libavfiler dev files in build stage (84171a9c0aa83bd4257b7bece193ba67c6677b34)
- Link against libavfiler in Makefile (fe2a7e7bd6569fe2c471160e4f3b7aa210801139)
- Add __attribute__ ((unused)) to fix warning. (a8eef11f7beb4ad2644745a1e6882038e555c6d1)
- Use vaapi scaling filter when using vaapi hw accel. (59dbfb32313a458fc53154c338c11de55734e760)
- Add va-driver packages to slim image (bfc96139e81dc0350b3397f7ced89e2118052445)
- Add EXOPTICON_HWACCEL_METHOD default to .env (a90bf966825a792a944a2d35a96f544a980903c3)

## 0.22.1
2021-12-30

### Fixes

- Add vaapi hwaccel libraries to runtime images (2db7cf88f11d4f23156b981dd74c4f5235809a60)
- Add non-free repo (acf0d35538fa7c0c30856a0fe83639bcff1ffc40)
- Add non-free repo (1db3e1266ba3330c7c8683eb15bc8c5cbc82cc15)
- dynamically select codec when using vaapi (c1bbe6c9c98fbeb8055c55b49a427d39073c40c8)
- Add initial captureworker benchmarks (e3a5d5708a1b9754e2e0b72efa805926bd3beafc)
- Add docker-compose overlay for vaapi (f160afb8a89e71a6123d2908540de2550ab7d26d)
- Fix exserial benchmarks (58cfec59f92ef246bda70d5ec307ac8e738f932b)

## 0.22.0
2021-12-22

### Features

- Add vaapi hw accelerated codecs to captureworker (d13c03e8395a10202b9ab62a2f5b26b1348a94e7)

### Fixes

- fix library ordering in cworkers Makefile (cde8b0aeee7c1792c05c5ca63bdeefc482965509)
- fix EXSERIAL_LIB path (aa0404df9f92c877f0a94073efd966dce19ed003)
- Implement qsv transcoding (9df140cb18a9b719cdd41a91f080fc300f979673)
- Fix default CARGO_TARGET_DIR (07ea40c37f3bdb8175380c5883eded173d8b3d0e)

## 0.21.5
2021-11-23

### Fixes

- Move library args to end of command to appease linker (268894ba0868789a2fb9358555e4a379e3074d62)
- Upgrade to bullseye image (6009a9bef702071cca24d85ba6c821c20aa6e80c)

## 0.21.4
2021-11-11

### Fixes

- Add importlib-metadata package to fix dvc install (730a036d2461a3ad99b2f6963b5de7dd5f6542b0)

## 0.21.3
2021-10-25

### Fixes

- Remove all event_observations attached to deleted events. (42e5edaaafb2f1e5ab5e7c6da543e6011d0c8c58)
- formatting (040cf03b5faaaa00e29c41f284fc638dc18e0378)

## 0.21.2
2021-10-25

### Fixes

- Delete all events attached to old video units (cfc8a7d834b09993d52d11d67e108e9227162d08)
- Also delete events without associated observations (bff501aba291b3d0e681d40f8c6ae9df230a1081)

## 0.21.1
2021-10-25

### Fixes

- Add file deletion logging (d96a49e483ca58f394e209a9ebfb776e1bb355a4)

## 0.21.0
2021-08-31

### Features

- Add frame skipped metrics to AnalysisActor (13e321caf285e81b3400723cf7216fd7f26fce03)

### Fixes

- initialize capture actor restart count to zero (179bd332ce18e0fb8a749e558b38eb2b176ccba4)

## 0.20.0
2021-08-31

### Features

- Add observation and event count metrics (068f9c72cf7e1468e449248f116d068b985bc2da)
- Add frame_count metric to CaptureActor (ac85bf76b5281a051aa15a78160483b6e74295f0)

### Fixes

- Switch analysis actors to AnalysisActorMetrics (e4ebd2e821fd965f3bf9c06acc5d72517ee8612b)
- change second if let into else (908491222bdc3141cd55d865b392968f62e5e481)

## 0.19.3
2021-08-31

### Fixes

- add metrics flag to .env.prod (c4b379aeb9c6c74c711c0f8558a2e48f8e4ff748)

## 0.19.2
2021-08-31

### Fixes

- enabled metrics in prod (2f5a78b10138a15f936298cdce97f43ed3623d2b)

## 0.19.1
2021-08-25

### Fixes

- Set first_pts when creating output file (3a1d728b785fc059b758cab534c52c59e7c9f59d)

## 0.19.0
2021-08-25

### Features

- Add postgres metrics at /dbmetrics route (2e661681c7f307e113ff2a4a7715b8179c9705b4)

## 0.18.18
2021-08-24

### Fixes

- persist non-motion observations before forwarding (5aa547fe4251c35cb6ec26d2bbe405d79df47526)

## 0.18.17
2021-08-23

### Fixes

- stop saving motion observations to db (2e845e09a9fae3b3c2c31e7830458406c4472770)

## 0.18.16
2021-08-20

### Fixes

- Upgrade to node 14 (73c0a7d72de01a0529097db864929afa30b0307a)
- upgrade to Angular 12 (12f71bfe3dec1396b5a3056ff1f31fad2f8f681a)
- Update dependencies (69ee76af78aa8c10a3ea7c2d98d7639de602ab96)
- Remove redundant --prod argument (cbd1a467439dd96f45103b3bb54dabf9c175371f)

## 0.18.15
2021-08-18

### Fixes

- Include snapshot size when calculating video unit size (569272a853c4efe749e7f81e58a2c721f6ab0095)

## 0.18.14
2021-08-13

### Fixes

- Expose port 3000 in docker-compose (40881b721567a0ec9ce7acdd817bbfaf4d69d373)

## 0.18.13
2021-08-13

### Fixes

- Remove empty lines in continuation (06eb71264c503aa98469611ca80392caf39de99d)
- switch docker-composer.web to Caddy (c630420283952c06e0e63d3817adb05eb43a5f16)
- Add env var to enable metrics (ae38f37cff688d69314359a17781cdf70220a564)

## 0.18.12
2021-08-05

### Fixes

- Disable unused analysis instances (dadaa9dfa24c9d7454ccc389f9938f7c2610661f)
- Remove unnecessary clone() (ba6a592f72b394f0400b8cf357234f4c9f854fde)
- Reconfigure cargo-make to run clippy and check formatting (59d1a7521c8ee18c63a934463a69af1dcbf0daf0)
- run build-release in ci-flow (3f11e0dd8b25a7d6982a778b80a46179d4a41414)

## 0.18.11
2021-08-02

### Fixes

- Create slim docker image (8cbd471c9bd89e43e4e3bc33d3061e20ac27838f)
- merge dockerignore (bd593456748025f95550c56dc10033e9ee73656e)

## 0.18.10
2021-07-28

### Fixes

- Update rust deps (ffcb29ab50a5a27792a1e2f90af310ec375263c8)

## 0.18.9
2021-07-28

### Fixes

- Update to actix_web 3 (3ab080a3f352d5fb0259eccc88d4b9c382408745)
- Bump libedgeputmax1 version to 16.0 (0820f0e773c44c9bfad60f1622469957248f507c)
- Specify cargo-make version to install (62ea3592e943fbe5153d854f5d63234e3e0d94d2)
- roll cargo-make version back to 0.32.13 to fix build (26790d52daee271b275cda8b5ecafb82523e27b3)

## 0.18.8
2021-07-25

### Fixes

- restrict yolo worker to person and dog labels (21ce1439edc614c472680ee78d5cb400c80da338)

## 0.18.7
2021-07-25

### Fixes

- Insert event analysis instance (20354ee76558f07492a7711251ef830008c09726)

## 0.18.6
2021-07-24

### Fixes

- unquote RELEASE_VERSION (3d23372e0fee28d467610335722081b8180259aa)
- Disable all build stages except deploy (ab95f432e0d286552a1598e4fe871ffb83d0ff40)
- Move pull command to single line (10d485fdcced11fd2fc546b12748f0c93d8beee8)
- Pass env to docker-compose as env-file (0991766ef4c07a764ae6c1330165947250a154e9)
- Add RELEASE_VERSION to env file (975b675c6ae7403b3b7937fa80d20cbe27700e59)
- fix device rule formatting, remove extra space (61bdefb91250519b0d7f9969a1951345d8f29ccb)
- Add exported ports to docker-compose.yml (6b26771e8624763eaff5399fef0eb5d114e0c455)
- Pass RUST_LOG envs to docker container (ff02da3d90053b6c7af1ebd6d6ef604296114ab1)
- Pass RUST_LOG env variable (78f47a650504ed9ee913c34c9446b9eb7692cccc)
- reenable build stages (ac90875f4faba2671ee383198bfb1d985e1a3757)
- Add analysis_engines to migrations so they cleanly apply (8d5c20e1d323bdb49537f8820c384e72b82a8aac)

## 0.18.5
2021-07-24

### Fixes

- Print value of RELEASE_VERSION in deploy (3c820b346dbe33f285cdfc94b0faf3a1fce48811)
- Remove explicit env params and add to dockerignore (36432aa7633528ae27a6f479bdecdc785593f6b8)

## 0.18.4
2021-07-24

### Fixes

- Add RELEASE_VERSION env to pull step (a05417bbd918e215036601aa7f94b95e35aad3cf)

## 0.18.3
2021-07-24

### Fixes

- Load release_info during deploy and add docker/ to dockerignore (596ed6e48168783d067668f21d31175eba5557e2)

## 0.18.2
2021-07-24

### Fixes

- Switch deploy image and add .gitlab-ci.yml to dockerignore. (83e1c15e539aa80c2a4b67dbbe43ff2994af5655)

## 0.18.1
2021-07-24

### Fixes

- Initial docker-compose deploy implementation (52ac03b9ccb14d09eff2f49e45863cb0b7249625)

## 0.18.0
2021-07-24

### Features

- Add event worker and endpoint (3da1ced5c7928570ae6be4268d1bf4ec52daf77f)
- Add Event api and event-list component. (d0c95ab646be5f4d57887ef4aafcb30c1d1b9c74)
- Add event api routes. (9bdb6db5a490e1a3390fb573cca5dfa695cdb117)
- Add EventList component (bcf379fa3b76f2cf1503ebe84bf8d9b81e0bc417)
- EventList: lock events to 16:9 ratio and show timestamps (56ffd43b127a2c7f1b156efe99c557e9d9028835)

### Fixes

- Add benchmark tests (31dcc2fe3432e684757e6090eba7b7457d9990f0)
- prevent creation of duplicate AnalysisSupervisors (135df35eea35687b99da1a990cc5b1a37b8f1ea2)
- update frigate version and fix coral analysis (57aec64cd566b53088a59a94e7a32b0966f8a1d6)
- take frame before scheduling future (1ec1f09beb50ef224f654bdf23df29e905289677)
- Update frigate submodule (5a21d73317ec3c877dfbbfa98633b70989256a3f)
- correct ports in metric docker-compose.yml's (d6d3e18920aa317690f610292c391161c8816617)
- Add analysis_offset to CameraFrame (c7ed2e62467ee8f5917c62eaeeef8880f26883b7)
- Update frigate submodule (bf84cad88da4794a3dae689bbf41cce1a908878c)
- format web code (cb69ec8dab02c7bd14255ea29efcaddb9e53a713)
- Add User service (5960a795b6f8af5b3db3a32e776a43a440b1a514)
- Make event views a fixed 16:9 aspect ratio. (b967b698801fc077e5493ff2929621ffb8ab0e91)
- Change label for top events button. (51eef9c9177c1e08aa62667d31d8c76511a58377)
- get_snapshot, only fail on empty stdout when writing to stdout. (a857dfd9ff011624ee06493b10d53e69df790210)
- Delete all events without EventObservation children (7fd38cb5a27e62bab322a773b76d136aa0f9198a)
- Add license header (0d22559a0563d387d84c24c3ff664741fc1b6b1e)
- Fix formatting (9db70ba2814320f680b8fbb4d45b98dd989aee5e)
- Bump go-semrel-gitlab version (d96ddbbbaab10bb7fe1c370ded3699389cc7257f)

## 0.18.0
2021-07-24

### Features

- Add event worker and endpoint (3da1ced5c7928570ae6be4268d1bf4ec52daf77f)
- Add Event api and event-list component. (d0c95ab646be5f4d57887ef4aafcb30c1d1b9c74)
- Add event api routes. (9bdb6db5a490e1a3390fb573cca5dfa695cdb117)
- Add EventList component (bcf379fa3b76f2cf1503ebe84bf8d9b81e0bc417)
- EventList: lock events to 16:9 ratio and show timestamps (56ffd43b127a2c7f1b156efe99c557e9d9028835)

### Fixes

- Add benchmark tests (31dcc2fe3432e684757e6090eba7b7457d9990f0)
- prevent creation of duplicate AnalysisSupervisors (135df35eea35687b99da1a990cc5b1a37b8f1ea2)
- update frigate version and fix coral analysis (57aec64cd566b53088a59a94e7a32b0966f8a1d6)
- take frame before scheduling future (1ec1f09beb50ef224f654bdf23df29e905289677)
- Update frigate submodule (5a21d73317ec3c877dfbbfa98633b70989256a3f)
- correct ports in metric docker-compose.yml's (d6d3e18920aa317690f610292c391161c8816617)
- Add analysis_offset to CameraFrame (c7ed2e62467ee8f5917c62eaeeef8880f26883b7)
- Update frigate submodule (bf84cad88da4794a3dae689bbf41cce1a908878c)
- format web code (cb69ec8dab02c7bd14255ea29efcaddb9e53a713)
- Add User service (5960a795b6f8af5b3db3a32e776a43a440b1a514)
- Make event views a fixed 16:9 aspect ratio. (b967b698801fc077e5493ff2929621ffb8ab0e91)
- Change label for top events button. (51eef9c9177c1e08aa62667d31d8c76511a58377)
- get_snapshot, only fail on empty stdout when writing to stdout. (a857dfd9ff011624ee06493b10d53e69df790210)
- Delete all events without EventObservation children (7fd38cb5a27e62bab322a773b76d136aa0f9198a)
- Add license header (0d22559a0563d387d84c24c3ff664741fc1b6b1e)
- Fix formatting (9db70ba2814320f680b8fbb4d45b98dd989aee5e)

## 0.17.0
2021-04-19

### Features

- Hide top menu when not logged in (dbc2e318e8a2478183b9c4187bc2762a08b84af0)
- Implement prometheus metrics for capture and analysis actors (06e34317c55b1ed12dcf88bca9d6606f48d76642)
- Add prometheus + grafana metrics (a5cc052f85f6f138f82421045be8be2c7e6849de)

### Fixes

- remove reject count for now (61a69026197cb2fc6d6fc889f0816ef7500220be)
- Fix missing menu on reopening without login (061ad82d8bddc80dff5df9f40fce3936b058ef85)

## 0.16.0
2021-04-01

### Features

- perform object detection on motion slice of image (ba6192020f7de681579163a22a11e65089e13eba)
- attach level to logs from analysis worker (70526a279c06b3d5ab9a46c065e63bec5993622c)

### Fixes

- Switch to cuda runtime (b5facda41f1c5ce1efe3e8763a45481b5b416e6a)
- Add comments to .env file (58f7eb92adec1b7f8eb953396c3a72441fd0642c)
- capture_actor: log stream closes as debug (5e7206d9890e698bf72db5cf6a69984c20345d63)
- change README to run make ci-flow so web assets are built (956a4c3c3fcda57e0c9fbd1dd81ff4407a7dc1d7)
- Add submodules to clone command in README (c27479af4e1b07e57c8da57bd1d76af61683da56)

## 0.15.1
2021-03-06

### Fixes

- fix sycning capture actors (eed80eba5b56a7cb44bcf9b781587a9205025ee9)
- Set dvc version to 1.11.16 (3de51329f5c9311ca8c31196de549c9fc1156255)

## 0.15.0
2021-03-06

### Features

- Add default yolo and coral obj analysis instances (7638ea8f6006b73cd6540ee4bcbb27ccb72c2d4a)

## 0.14.0
2021-03-04

### Features

- Add simple analysis configuration and angular form. (69ce252f4a6ee73281b8aaa0105e62e1725b301b)

### Fixes

- Add missing "tag" field to CameraSource (31dfa49b6b169c37a6ae7d033ac3278ef49bde7b)
- Enabled Docker buildkit (803b3eb3a96dd14e38d9c148d614672b5c1c4283)
- pull env variables into compose file (77702faa24c8d861fe11a7756ca054ad83b9aee3)

## 0.13.5
2021-02-26

### Fixes

- remove print statements (2f037e7073bdffc594e5e5bcbfc82ba4a0452c56)

## 0.13.4
2021-02-26

### Fixes

- enable Docker buildkit (5ba38bd0b6a3cf69163b67b130d4f9d1be43e271)

## 0.13.3
2021-02-19

### Fixes

- only start enabled AnalysisInstances (a5e9eb43b1ea0b5434b92e4089b68ca0b9cdec7d)

## 0.13.2
2021-02-18

### Fixes

- Document creation of dev environment (faa96e2c0f8fcc461c88e5053f70a5ef60107382)

## 0.13.1
2021-02-17

### Fixes

- Refactor docker-compose files to use single .env file (f14b3721b8aba7891d3f6a9c7912644bdc53eb6b)

## 0.13.0
2021-02-16

### Features

- Make webmanifest template (c22f721ef2288dbc24bd8b003d59adc15c2e1af1)

### Fixes

- Add python3-dev as a dependency to fix dvc build (0c54da7d97e85fd97a61404b1681b12a9cf18639)

## 0.12.0
2021-02-06

### Features

- Add docker-compose files (1389b25451255ceb19b43cd2fcac9c4a79018d3f)
- Implement initial docker compose files. (dd49eedf30c47b6f1a0ba109b6e64826d46f813f)
- broken subregion detection (57faf5df61faab0fb8c3c7b7246ae964b992bbac)

### Fixes

- Include env variables in runtime and development stages (54936b2d62a889877158def1d5e68a181c704cba)
- Add b2 dvc remotes (32fe16e430bfa80a6b66d77c9902178f5d07a1a0)
- install dvc s3 support (197dfa75e7638ddbc4ec37f35c8ca733509c8ec0)
- Change default dvc remote to http (5f60d76de9c670690ce11076a4f30caff527b0c5)

## 0.11.0
2021-01-18

### Features

- Initial main menu (f4d8ec67cc8c8c4af91cca9b56eccfe41e655947)
- Add camera form (b496ffebb4ba031322f12f9a268190d2d1b0a6b4)
- Add password confirmation to cli user creation (dfda99b11a7ff44ff2301379c4225b69de2255f3)
- Allow camera detail form to submit new cameras. (d71e779feaf1cfc90bef52e0a4cf872712b43b6e)
- Sync cameras when camera is added or updated (59a459a3764f3d19ab8b0f012a665f17b876a272)

### Fixes

- remove unnecessary logging (55818e82eb4399181b43b31c3378381dad50b98f)
- include python3-gi in dev and runtime images. (8c7c0cfa4182ba83a8704f0a5bc8e0dc17c0b990)
- Restart captureworkers when a camera is updated. (0faf54abf2dca714f7ce00df42b82a11324cd503)

## 0.10.2
2020-12-31

### Fixes

- fix cuvid library name (a9b61046c2a295658d65e49def401ec5bd77777b)

## 0.10.1
2020-12-31

### Fixes

- Map libcuvid libraries into production container (0e488b08b06a3230981355965449ef926a6b746b)

## 0.10.0
2020-12-30

### Features

- cuda video decoding (bc748373985d7c16a106af6584f7dae601ae0fae)

### Fixes

- fix ffmpeg build in Dockerfile (5b6667053b0dd50f7f8926472780d7e165592b35)
- Add ffmpeg to runtime (b8dd5c4be548a4af50c20c08047ce3d3878215cf)
- remove debug messages from exvid (3db5449e286f8cbca0852dca80b0342678c321ed)
- Add pycoral and pillow to runtime environment (59e239d016d1b5aff83959ac6a35b36e9cfc7526)

## 0.9.0
2020-12-17

### Features

- Flush packets and use direct io (54790da79a3023306d622da697bdeac9d7d4880e)

## 0.8.0
2020-12-12

### Breaking changes

#### Switch VideoUnit to uuid for primary key (5d327cc824f106b1fb47e6f30d7078100d980dd7)

VideoUnit now uses a uuid for id

### Fixes

- Change video_unit table to use a uuid primary key (45f9cfed3bcd9b3b8a05681a2e5cd433f3566e61)
- Remove Cargo.lock from .dockerignore to fix build (475b838a9530c75f3ee11174a8353c782327a944)
- Add Cargo.lock (6ce486d79824f483c63856d74cd6058a2bdc4e5f)

## 0.7.0
2020-12-04

### Features

- Implement ServiceWorker and initial PWA (2041e28981a474b201a398550f5dd082fb7a778b)
- Add new header (8d77cbb29a45703d86fa32137103f28c96e6c0b3)
- Add HOME button to header (d98457dc7bfea625d4ae3858433c5ad3bc01cddb)

### Fixes

- Rework authentication flow to live inside Angular (47341d8ef6c583045924bbd603dd6071eff73ca9)
- **web:** fix formatting (efb7c962b3904a6eac4a2de026da760afc1ebd29)
- Build with production environment so web manifest is included (741c79530679e41c7e4d6e2c707e30fc69d99a82)
- style pwa splash screen (25382449391e4a80d8ff50b2dd9e8df5ab0bd9b6)
- fix layout width (d0b23e8052202e7f0a85fc0efcd02e4e4e655585)

## 0.6.2
2020-11-30

### Fixes

- chore: Add GPLv3 to exopticon rust source files. (0e83f65c98056ac809683be2dd7d10df7d683049)
- chore: Add GPLv3 header to web source files. (83f1ca5d0751c1b81125cbbbd3b15d58161ce86a)
- chore(web): fix formatting and add check-format to build. (0d5afd69f1a469c9c1568d73533d644f24c1e1ec)
- chore: Add GPLv3 header to playback sources (3375e2bf53db289c1268c7cad052b34ec4359134)
- chore: Update workspace GPLv3 headers (2f50d1482f0be7196e0ac342c9361d09426caf23)
- chore: Add GPLv3 license (ae77ee9e3b2888a5ea7cbefe1b29589b89b56389)
- chore: Change onvif license to GPLv3 (a948ecf27fed162cab18eef064a6fed013408311)
- chore: Update Cargo manifests to GPLv3 (8f387e1650df1bc51c087a499e93bea42dfea247)

## 0.6.1
2020-11-24

### Fixes

- **cicd:** Move build after pulling dvc models (6e0e8086a14ef8e920872d9d2a6d4e5d349a1b12)

## 0.6.0
2020-11-24

### Features

- **analysis:** Implement initial coral analysis worker (670ab95d2f7bf80f89401f2f4ba80cd924a2a091)
- **analysis:** Merge branch '27-coral-analysis-worker' (0afca87334dc1ae7625259a7b40f8e5f01f16d82)

### Fixes

- **cicd:** Add ssd mobilenet model dvc pull (4897b4d7f8c8ce547a5b158421aa9487995d1e4a)
- **cicd:** Install xz-utils to unpack libedgetpu deb (0ce5371e496a5d6872d49da36000ed58ae556682)

## 0.5.9
2020-11-23

### Fixes

- Pull yolov4-tiny model from dvc when building image (e3cbfc59ff974bdc0cceff446729ec888537e776)
- **cicd:** Fix ci/cd image (12879ddb805806cfe5f815584f03bbc89bb8e310)
- **cicd:** try this image (e49effddc135b88d02f8be81ea7a6127b85fa3e6)
- **cicd:** docker build on prerelease (3d486f5ddf34162c50d0377588ea4c42022da82b)

## 0.5.8
2020-11-15

### Fixes

- refactor(web]: Upgrade to Angular 11 (0fff31a003af4652dd09957a608e6ad7e840befc)

## 0.5.7
2020-11-12

### Fixes

- build: merge dockerfiles (9a7ff3d5cf3d266cf20208022dca2890f67540f7)

## 0.5.6
2020-11-12

### Fixes

- build: recombine the Dockerfile into one. (5841423f63d115c57bfe4603a072ae5ca771aca3)

## 0.5.5
2020-11-10

### Fixes

- **docker:** stop purging python3-opencv (eb4bf56052c2ca067428e6e1f35c0f1e20953a3a)

## 0.5.4
2020-11-10

### Fixes

- **web:** Add dist and node_modules to dockerignore (4d75737580197b38ff365847339cc7b254863a35)

## 0.5.3
2020-11-10

### Fixes

- Add SECRET_KEY to prod deploy (2f05ddd8b817e30a3bbafbb312c5b868ee4f52a9)

## 0.5.2
2020-11-10

### Fixes

- playbackworker: fix eof detection on stdin. (a6d68280bf35dbf62a8764063473e9e6424e92db)

## 0.5.1
2020-11-06

### Fixes

- fix dockerignore file (75672a9f301337c799e28e53a80d3fab673a3983)

## 0.5.0
2020-11-03

### Features

- Switch tslint to double quotes. (8bc332ca7bada8c5cb1a2468ecf4b2b25287e2aa)
- Initial CameraPanelService implementation. (26fb9079e1eb2bad5bece794b6462e4a20c9ef67)

## 0.4.0
2020-10-16

### Features

- **cicd:** Add production deployment to the ci/cd pipeline. (013f50f635b485330e7d5275f725f0c5f6b13b95)

### Fixes

- **cicd:** Add deploy stage. (0d083b9825d99adf9237e23008748304121d4e26)
- **cicd:** hack to get version into deploy stage. (d3c687363ab7e3ee1a48e8c1e30bd22c4ab7f62a)
- **cicd:** fix ssh key variable name (7b5767dc62d417005f9b19a8e70c161c28122dff)
- **cicd:** Change library 'find' command to not use shell globbing. (9b22786c0f96ce5739f95582f7ef20d1d670d0fd)
- **cicd:** Remove old container during deploy. (758d31c212cd9e02e77edfa1cde59701297f10e4)
- **cicd:** Add DATABASE_URL and ROOT_URL vars to docker run. (1f60077cbb298b465281291c4335ce0b6ccdbe57)
- **cicd:** Map env variables because they are exported to remote. (5b318470164e1ac8877c9a2b1284ff0faa285223)
- **cicd:** Make sure deployed container runs in background. (d71c820f71c13bb07cf9cad366d53babce6f9c6a)
- **build:** Build web assets before release build. (586becb106c9a9cd6d2d4a7f6f2541a420d42090)

## 0.3.0
2020-10-15

### Features

- Embed migrations and run them at startup. (bbc5f49303f41cde9b8c86264bbc25c3d19deafa)

### Fixes

- **db:** Fix migration syntax. (6b5456c85e975835ce8d91b395ac151bc43e975d)
- Remove a redundant clone. (a7ea9f1f465af6f819cc64466b688f7cfc45cf4f)

## 0.2.0
2020-10-14

### Features

- **web:** Update the project to Angular 10. (b9f3cbe36e1dc8c6fa150ce8435b0781b6aed33a)

## 0.1.0
2020-10-12

### Features

- **web:** Remove the angular-inviewport dependency. (752943bdc0f22304f9692b73dd34b501e6f76946)

### Fixes

- **web:** Remove logs from ElementVisibleService. (3035c7c4c8f4766bd4b42e5377400971fce3f6ea)
- **web:** Remove an unused method in CameraView. (52919adc2d509806b2d24eadb26c49c0bf7b615f)
- Remove clippy::restriction lints (3e0fa8f58578f026de6d6ce512af9dc3c8a753c0)
- Fix clippy lints for rustc version 1.47.0 (287fb5b2f8e5f7928fa581f371cfbec992a30e81)
- Add link to libcuda.so stub. [skip-ci] (e3b182a2b583ffc1e61da3a954a5c1b718c12a2e)
- **cicd:** libcuda.so.1 link is created in the base image now. (8a76bf4c1f15ce072670a0b066efd0fc0754fad1)
- **cicd:** Remove _the other_ hardlink to the libcuda stub. (57f1febd07f30c43fbc04c02a6adfc5573e23306)

## 0.0.5
2020-10-12

### Fixes

- Remove clippy::restriction lints (6c9c050d19d4d61027c32cf5b30eca15629e8432)

## 0.0.4
2020-10-11

### Fixes

- Remove clippy::restriction lints (8f34d7b040ea9b76ad9f6ae375787bfa05be2b7c)
- Fix clippy lints for rustc version 1.47.0 (0c55fcf7b8bb3d3e2cc6eda50f2bbdaa9b9a5b5c)
- **cicd:** libcuda.so.1 link is created in the base image now. (479c670f9c2e328b5aceca562bb1e67993452023)
- **cicd:** Remove _the other_ hardlink to the libcuda stub. (29f199c1e5b8c8b7d993499b52d265f2a7dc9ef8)

## 0.0.3
2020-10-10

### Fixes

- Add link to libcuda.so stub. [skip-ci] (8adbddb57838a97e42fe9d26df8a0be2a969d209)

## 0.0.2
2020-10-09

### Fixes

- chore: make each build stage run in a docker image (05d53bea4678c9dcf75ddf230e9e4262d3de21a4)
- Merge branch 'master' of gitlab.com:dmattli/exopticon (cd2dcace77539c6ab5d6e29bf2d402565cee2aae)
- chore: bump patch on build (40e7bdba4cb85e7102317145fdb05d8889f6adcb)
- chore: Add bump-patch to release commands. (fa5049862733acaed34128b874a8d135ce92c3d5)

## 0.0.1
2020-10-08

### Fixes

- make link to libcuda.so in Dockerfile (bd14767c7708f0adeedaeaa1d93d174e9e49e72c)
- Add python3-wheel to runtime image to fix imutils pip (17d0e45a5718794dbb8361ed34877481aac9209f)