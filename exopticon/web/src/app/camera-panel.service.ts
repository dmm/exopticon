/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

import { Injectable } from "@angular/core";
import { concat, defer, forkJoin, fromEvent, of } from "rxjs";
import { map } from "rxjs/operators";
import { Camera, CameraId } from "./camera";
import { CameraGroup, CameraGroupId } from "./camera-group";
import { CameraGroupService } from "./camera-group.service";
import { CameraService, PtzDirection } from "./camera.service";
import { WebrtcService } from "./webrtc.service";

const ALL_GROUP_ID = "34e79812-df14-4773-a9f4-f766c799aa62";

class PanelCamera {
  camera: Camera;
  inViewport: boolean;
  enabled: boolean;
}

enum SelectionMode {
  Touch,
  Mouse,
}

@Injectable()
export class CameraPanelService {
  // value between 0.0 and 1.0 inclusive representing how much of each
  // CameraView has to overlap the viewport to activate.
  intersectionThreshold = 0.1;

  //
  // Public binding properties
  // These public properties are exported for templates to bind against.
  //

  // cameras
  cameras: Camera[] = [];
  cameraDesiredState: boolean[] = [];

  // panel column count
  columnCount: number = 1;

  // panel row count
  rowCount: number = -1;

  // camera offset
  offset: number = 0;

  // id of currently selected camera or -1
  selectedCameraId: CameraId = null;

  // Whether camera was selected by touch or mouse
  selectedCameraMode = SelectionMode.Mouse;

  //
  keyboardControlCameraId: CameraId = null;

  // Active camera group id, 0 for all cameras aka no group
  activeCameraGroupId: CameraGroupId = null;

  activeCameraGroupName: string = "ALL";

  desiredCameraGroupId: CameraGroupId = null;

  nextCameraGroupId: CameraGroupId = null;

  prevCameraGroupId: CameraGroupId = null;

  //
  // End public binding properties
  //

  // Unsorted cameras
  private unsortedCameras: Map<CameraId, PanelCamera> = new Map();

  // Camera groups
  private cameraGroups: Map<CameraGroupId, CameraGroup> = new Map();

  // page visible observable
  private pageVisible$ = concat(
    defer(() => of(!document.hidden)),
    fromEvent(document, "visibilitychange").pipe(map(() => !document.hidden)),
  );

  private pageVisible: boolean = false;

  private setCameraTimeout = null;

  private projectCameraTimeout = null;

  constructor(
    private cameraService: CameraService,
    private cameraGroupService: CameraGroupService,
    private webrtcService: WebrtcService,
  ) {
    this.pageVisible$.subscribe((visible) => {
      this.pageVisible = visible;
      if (this.pageVisible) {
        this.setCameras();
        this.webrtcService.enable();
      } else {
        // This should disable cameras when page hidden.
        this.projectCameras();
        this.webrtcService.disable();
      }
    });
  }

  private rotateArray(arr: Array<any>, length: number): Array<any> {
    if (arr.length === 0) return [];
    arr = arr.slice();

    if (length > 0) {
      for (let i = 0; i < length; i++) {
        arr.unshift(arr.pop());
      }
    } else {
      for (let i = 0; i < Math.abs(length); i++) {
        arr.push(arr.shift());
      }
    }

    return arr;
  }

  private registerSetCameras() {
    if (this.setCameraTimeout == null) {
      this.setCameraTimeout = setTimeout(() => this.setCameras(), 2000);
    }
  }

  private setCameras() {
    this.setCameraTimeout = null;

    forkJoin({
      cameras: this.cameraService.getCameras(),
      cameraGroups: this.cameraGroupService.getCameraGroups(),
    })
      .toPromise()
      .then((res) => {
        let cameras = res.cameras;
        let cameraGroups = res.cameraGroups;

        this.unsortedCameras.clear();
        cameras
          .filter((c) => c.enabled)
          .forEach((c) => {
            let camera = new PanelCamera();
            camera.camera = c;
            camera.inViewport = false;
            camera.enabled = true;
            this.unsortedCameras.set(camera.camera.id, camera);
          });

        this.cameraGroups.clear();
        cameraGroups.forEach((group) => {
          this.cameraGroups.set(group.id, group);
        });

        this.projectCameras();
      })
      .catch((err) => {
        console.log(err);
        this.registerSetCameras();
      });
  }

  private projectCameras() {
    if (this.projectCameraTimeout === null) {
      this.projectCameraTimeout = setTimeout(
        () => this.realProjectCameras(),
        200,
      );
    }
  }

  private realProjectCameras() {
    this.projectCameraTimeout = null;
    if (this.desiredCameraGroupId === null) {
      this.desiredCameraGroupId = ALL_GROUP_ID;
    }

    if (this.activeCameraGroupId === null) {
      this.activeCameraGroupId = ALL_GROUP_ID;
    }

    this.setCameraGroup(this.desiredCameraGroupId);
    let groupCameras: PanelCamera[] = new Array();
    let cameraGroup = this.cameraGroups.get(this.activeCameraGroupId);
    cameraGroup.members.forEach((cameraId) => {
      const c = this.unsortedCameras.get(cameraId);
      if (c !== undefined) {
        groupCameras.push(this.unsortedCameras.get(cameraId));
      }
    });

    this.nextCameraGroupId = this.nextCameraGroup();
    this.prevCameraGroupId = this.prevCameraGroup();

    let cameraCount = 0;
    if (this.rowCount < 1) {
      cameraCount = this.unsortedCameras.size;
    } else {
      cameraCount = this.rowCount * this.columnCount;
    }

    this.cameras = this.rotateArray(
      groupCameras.map((c: PanelCamera) => c.camera),
      this.offset,
    ).slice(0, cameraCount);

    // this isn't great...
    let activeCameraIds: CameraId[] = new Array();
    this.cameraDesiredState = this.cameras.map((c) => {
      let p = this.unsortedCameras.get(c.id);
      let active = p.inViewport && p.enabled && this.pageVisible;
      if (active) {
        activeCameraIds.push(c.id);
      }

      return active;
    });
    this.webrtcService.updateActiveCameras(activeCameraIds);
  }

  setRows(rowCount: number) {
    this.rowCount = rowCount;
  }

  setCols(colCount: number) {
    this.columnCount = colCount;
  }

  setOffset(offset: number) {
    this.offset = offset;
    this.projectCameras();
  }

  setCameraVisibility(
    cameraId: CameraId,
    intersectionEvents: IntersectionObserverEntry[],
  ) {
    let panelCamera = this.unsortedCameras.get(cameraId);

    if (panelCamera !== null) {
      panelCamera.inViewport = intersectionEvents.some(
        (e) => e.intersectionRatio >= this.intersectionThreshold,
      );
    }

    this.projectCameraTimeout;
    this.projectCameras();
  }

  ptz(direction: PtzDirection) {
    let camera = this.cameras.find((c) => c.id === this.selectedCameraId);
    if (camera) {
      this.cameraService.ptz(camera.id, direction);
    }
  }

  // Event Handlers
  touchCamera(cameraId: CameraId) {
    if (this.selectedCameraId !== cameraId) {
      this.selectedCameraId = cameraId;
      this.keyboardControlCameraId = cameraId;
      this.selectedCameraMode = SelectionMode.Touch;
    } else if (this.selectedCameraId === cameraId) {
      this.selectedCameraId = null;
      this.keyboardControlCameraId = null;
      this.selectedCameraMode = SelectionMode.Touch;
    }
  }
  mouseOver(cameraId: CameraId) {
    if (
      this.selectedCameraId === cameraId &&
      this.selectedCameraMode === SelectionMode.Mouse
    ) {
      // clear selected camera on second touch
      this.selectedCameraId = null;
    } else if (this.selectedCameraMode === SelectionMode.Mouse) {
      this.selectedCameraId = cameraId;
      this.keyboardControlCameraId = cameraId;
      this.selectedCameraMode = SelectionMode.Mouse;
    }
  }

  mouseLeave() {
    if (this.selectedCameraMode === SelectionMode.Mouse) {
      this.selectedCameraId = null;
      this.keyboardControlCameraId = null;
    }
  }

  setDesiredCameraGroup(cameraGroupId: CameraGroupId) {
    this.desiredCameraGroupId = cameraGroupId;
    this.projectCameras();
  }

  private setCameraGroup(cameraGroupId: CameraGroupId) {
    // check validity of cameraGroupId
    if (cameraGroupId !== null && !this.cameraGroups.has(cameraGroupId)) {
      return false;
    }
    this.activeCameraGroupId = cameraGroupId;

    if (cameraGroupId !== null) {
      let cameraGroup = this.cameraGroups.get(cameraGroupId);
      this.activeCameraGroupName = cameraGroup.name;
    } else {
      this.activeCameraGroupName = "ALL";
    }
    return true;
  }

  nextCameraGroup(): CameraGroupId {
    let ids = Array.from(this.cameraGroups.keys());
    ids.sort();
    let currentIndex = ids.indexOf(this.activeCameraGroupId);
    let nextIndex = currentIndex + 1;
    if (nextIndex >= ids.length) {
      // ALL group
      return ids[0];
    } else {
      return ids[nextIndex];
    }
  }

  prevCameraGroup(): CameraGroupId {
    let ids = Array.from(this.cameraGroups.keys());
    ids.sort();
    ids.reverse();
    let currentIndex = ids.indexOf(this.activeCameraGroupId);
    let prevIndex = currentIndex + 1;

    if (prevIndex >= ids.length) {
      // ALL group
      return ids[0];
    } else {
      return ids[prevIndex];
    }
  }
}
