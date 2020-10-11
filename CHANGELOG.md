# CHANGELOG

<!--- next entry here -->

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