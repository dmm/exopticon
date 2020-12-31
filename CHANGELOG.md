# CHANGELOG

<!--- next entry here -->

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