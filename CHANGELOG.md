# CHANGELOG

<!--- next entry here -->

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