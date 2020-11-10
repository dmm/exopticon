# CHANGELOG

<!--- next entry here -->

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