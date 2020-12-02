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
import { concat, defer, fromEvent, of } from "rxjs";
import { map } from "rxjs/operators";
import { Camera } from "./camera";
import { CameraService, PtzDirection } from "./camera.service";
import { CameraResolution } from "./frame-message";
import { VideoService } from "./video.service";

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
  selectedCameraId: number = -1;

  // Whether camera was selected by touch or mouse
  selectedCameraMode = SelectionMode.Mouse;

  //
  keyboardControlCameraId: number = 0;

  // Current resolution
  resolution: CameraResolution = CameraResolution.Sd;

  //
  // End public binding properties
  //

  // Unsorted cameras
  private unsortedCameras: PanelCamera[] = [];

  // page visible observable
  private pageVisible$ = concat(
    defer(() => of(!document.hidden)),
    fromEvent(document, "visibilitychange").pipe(map(() => !document.hidden))
  );

  private pageVisible: boolean = true;

  constructor(
    private cameraService: CameraService,
    private videoService: VideoService
  ) {
    this.pageVisible$.subscribe((visible) => {
      this.pageVisible = visible;
      if (this.pageVisible) {
        this.setCameras();
      } else {
        // This should disable cameras when page hidden.
        this.projectCameras();
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

  private setCameras() {
    this.cameraService.getCameras().subscribe(
      (cameras) => {
        this.unsortedCameras = cameras
          .filter((c) => c.enabled)
          .map(
            (c) => {
              let camera = new PanelCamera();
              camera.camera = c;
              camera.inViewport = false;
              camera.enabled = true;
              return camera;
            },
            () => {
              // error fetching cameras
              setTimeout(() => this.setCameras(), 2000);
            }
          );

        this.videoService.getErrorObservable().subscribe((error) => {
          console.log("Caught error and restarting!");
          // TODO: Should we implement exponential backoff? We probably
          // need connection management in general.
          setTimeout(() => this.setCameras(), 2000);
        });

        this.projectCameras();
      },
      (err) => {
        setTimeout(() => this.setCameras(), 2000);
      }
    );
  }

  private projectCameras() {
    let cameraCount = 0;
    if (this.rowCount < 1) {
      cameraCount = this.unsortedCameras.length;
    } else {
      cameraCount = this.rowCount * this.columnCount;
    }

    this.cameras = this.rotateArray(
      this.unsortedCameras.map((c) => c.camera),
      this.offset
    ).slice(0, cameraCount);

    // this isn't great...
    this.cameraDesiredState = this.cameras.map((c) => {
      let p = this.unsortedCameras.find((p) => p.camera.id === c.id);
      return p.inViewport && p.enabled && this.pageVisible;
    });
  }

  setRows(rowCount: number) {
    this.rowCount = rowCount;
  }

  setCols(colCount: number) {
    this.columnCount = colCount;
  }

  setResolution(resolution: CameraResolution) {
    this.resolution = resolution;
  }

  setOffset(offset: number) {
    this.offset = offset;
    this.projectCameras();
  }

  setCameraVisibility(
    cameraId: number,
    intersectionEvents: IntersectionObserverEntry[]
  ) {
    let panelCamera = this.unsortedCameras.find(
      (camera: PanelCamera) => camera.camera.id == cameraId
    );

    if (panelCamera) {
      panelCamera.inViewport = intersectionEvents.some(
        (e) => e.intersectionRatio >= this.intersectionThreshold
      );
    }

    this.projectCameras();
  }

  ptz(direction: PtzDirection) {
    this.cameraService.ptz(this.keyboardControlCameraId, direction);
  }

  // Event Handlers
  touchCamera(cameraId: number) {
    if (this.selectedCameraId !== cameraId) {
      this.selectedCameraId = cameraId;
      this.keyboardControlCameraId = cameraId;
      this.selectedCameraMode = SelectionMode.Touch;
    }
  }
  mouseOver(cameraId: number) {
    if (
      this.selectedCameraId === cameraId &&
      this.selectedCameraMode === SelectionMode.Mouse
    ) {
      // clear selected camera on second touch
      this.selectedCameraId = -1;
    } else {
      this.selectedCameraId = cameraId;
      this.keyboardControlCameraId = cameraId;
      this.selectedCameraMode = SelectionMode.Mouse;
    }
  }

  mouseLeave() {
    if (this.selectedCameraMode === SelectionMode.Mouse) {
      this.selectedCameraId = null;
      this.keyboardControlCameraId = 0;
    }
  }
}
