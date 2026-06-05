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

import {
  ChangeDetectorRef,
  Component,
  effect,
  HostListener,
  NgZone,
  OnInit,
} from "@angular/core";
import { ActivatedRoute, Router } from "@angular/router";
import { AsyncPipe } from "@angular/common";
import { IntersectionObserverModule } from "@ng-web-apis/intersection-observer";
import { Camera, CameraId } from "../camera";
import { CameraGroupId } from "../camera-group";
import { CameraPanelService } from "../camera-panel.service";
import { CameraService, PtzDirection } from "../camera.service";
import { CameraViewComponent } from "../camera-view/camera-view.component";
import { WebrtcService } from "../webrtc.service";

interface CameraPanelFocusHistoryState {
  cameraPanelFocusPreviousFsPresent?: boolean;
  cameraPanelFocusPreviousFs?: "true" | "false";
  cameraPanelFocusAutoFs?: boolean;
}

@Component({
  selector: "app-camera-panel",
  templateUrl: "./camera-panel.component.html",
  styleUrls: ["./camera-panel.component.css"],
  providers: [CameraPanelService],
  imports: [IntersectionObserverModule, CameraViewComponent, AsyncPipe],
})
export class CameraPanelComponent implements OnInit {
  cameras: Camera[] = [];
  enabledCameras: Camera[] = [];
  enabledCamerasOffset: number = 0;
  fullscreen: boolean = false;
  error: unknown;
  private cameraVisibility = new Map<string, boolean>();
  private normalizingFocusedUrl = false;
  webrtcStatus$ = this.webrtcService;

  constructor(
    public cameraPanelService: CameraPanelService,
    private cameraService: CameraService,
    public webrtcService: WebrtcService,
    private cdr: ChangeDetectorRef,
    private route: ActivatedRoute,
    private router: Router,
    private ngZone: NgZone,
  ) {
    effect(() => {
      const focusState = this.cameraPanelService.vm().focusState;
      if (
        focusState.kind === "invalid" &&
        this.route.snapshot.paramMap.get("focus") === focusState.cameraId
      ) {
        this.ngZone.run(() => this.clearInvalidFocus());
      }
    });
  }

  getCameras(): void {
    this.cameraService.getCameras().subscribe((cameras) => {});
  }

  ngOnInit() {
    this.route.paramMap.subscribe((params) => {
      if (params.has("cols")) {
        this.cameraPanelService.setCols(parseInt(params.get("cols") ?? "", 10));
      }
      if (params.has("rows")) {
        this.cameraPanelService.setRows(parseInt(params.get("rows") ?? "", 10));
      }

      if (params.has("offset")) {
        this.cameraPanelService.setOffset(
          parseInt(params.get("offset") ?? "", 10),
        );
      }

      if (params.has("group")) {
        this.cameraPanelService.setDesiredCameraGroup(params.get("group"));
      } else {
        this.cameraPanelService.setDesiredCameraGroup(null);
      }

      this.cameraPanelService.setFocusedCamera(params.get("focus"));
      this.normalizeFocusedUrlIfNeeded();
    });

    this.route.queryParamMap.subscribe((params) => {
      if (params.has("fs") && params.get("fs") === "true") {
        this.fullscreen = true;
      } else {
        this.fullscreen = false;
      }

      this.normalizeFocusedUrlIfNeeded();
    });

    //    this.videoService.connect();
    //this.webrtcService.connect();
  }

  @HostListener("window:keyup", ["$event"])
  KeyEvent(event: KeyboardEvent) {
    const vm = this.cameraPanelService.vm();
    const cameraCount = vm.tiles.length;
    let offset = vm.offset;
    const focused = vm.focusedCameraId !== null;

    switch (event.keyCode) {
      case 27:
        // Escape
        if (focused) {
          this.returnFromFocus();
        }
        break;
      case 78:
        // 'n'
        if (!focused && cameraCount > 0) {
          offset = (offset + 1) % cameraCount;
        }
        break;
      case 80:
        // 'p'
        if (!focused && cameraCount > 0) {
          offset = (offset - 1) % cameraCount;
        }
        break;
      case 65:
        // 'a'
        this.cameraPanelService.ptz(PtzDirection.left);
        break;
      case 68:
        // 'd'
        this.cameraPanelService.ptz(PtzDirection.right);
        break;
      case 87:
        // 'w'
        this.cameraPanelService.ptz(PtzDirection.up);
        break;
      case 83:
        // 's'
        this.cameraPanelService.ptz(PtzDirection.down);
        break;
    }

    if (offset !== vm.offset) {
      this.router.navigate(
        ["./", this.merge({ offset: offset }, this.route.snapshot.params)],
        {
          relativeTo: this.route,
          queryParams: this.route.snapshot.queryParams,
          state: this.focusHistoryStateForNavigation(),
        },
      );
    }
  }

  merge(newParams: any, oldParams: any): any {
    let params = Object.assign({}, oldParams);

    for (var prop in newParams) {
      if (newParams.hasOwnProperty(prop)) {
        let newValue = newParams[prop];
        if (newValue === null) {
          delete params[prop];
        } else {
          params[prop] = newValue;
        }
      }
    }

    return params;
  }

  updateCameraViewVisibility(cameraId: string, visible: boolean) {
    console.log(`Visibility change: ${cameraId} ${visible}`);
    this.cameraVisibility.set(cameraId, visible);
  }

  focusCamera(cameraId: CameraId): void {
    const queryParams = {
      ...this.route.snapshot.queryParams,
      fs: "true",
    };

    this.router.navigate(
      ["./", this.merge({ focus: cameraId }, this.route.snapshot.params)],
      {
        relativeTo: this.route,
        queryParams,
        replaceUrl: false,
        state: this.buildFocusHistoryState(),
      },
    );
  }

  returnFromFocus(): void {
    const queryParams = { ...this.route.snapshot.queryParams };
    const focusHistoryState = this.readFocusHistoryState();

    if (focusHistoryState?.cameraPanelFocusPreviousFsPresent === false) {
      delete queryParams["fs"];
    } else if (
      focusHistoryState?.cameraPanelFocusPreviousFsPresent === true &&
      focusHistoryState.cameraPanelFocusPreviousFs !== undefined
    ) {
      queryParams["fs"] = focusHistoryState.cameraPanelFocusPreviousFs;
    }

    this.router.navigate(
      ["./", this.merge({ focus: null }, this.route.snapshot.params)],
      {
        relativeTo: this.route,
        queryParams,
        replaceUrl: true,
      },
    );
  }

  normalizeFocusedUrlIfNeeded(): void {
    if (
      !this.normalizingFocusedUrl &&
      this.route.snapshot.paramMap.has("focus") &&
      !this.route.snapshot.queryParamMap.has("fs")
    ) {
      this.normalizingFocusedUrl = true;
      this.router
        .navigate(["./", this.route.snapshot.params], {
          relativeTo: this.route,
          queryParams: {
            ...this.route.snapshot.queryParams,
            fs: "true",
          },
          replaceUrl: true,
          state: {
            cameraPanelFocusPreviousFsPresent: false,
            cameraPanelFocusAutoFs: true,
          } satisfies CameraPanelFocusHistoryState,
        })
        .finally(() => {
          this.normalizingFocusedUrl = false;
        });
    }
  }

  clearInvalidFocus(): void {
    this.router.navigate(
      ["./", this.merge({ focus: null }, this.route.snapshot.params)],
      {
        relativeTo: this.route,
        queryParams: this.route.snapshot.queryParams,
        replaceUrl: true,
        state: this.focusHistoryStateForNavigation(),
      },
    );
  }

  setCameraGroup(newGroupId: CameraGroupId | null) {
    this.router.navigate(
      ["./", this.merge({ group: newGroupId }, this.route.snapshot.params)],
      {
        relativeTo: this.route,
        queryParams: this.route.snapshot.queryParams,
        state: this.focusHistoryStateForNavigation(),
      },
    );
  }

  private buildFocusHistoryState(): CameraPanelFocusHistoryState {
    const queryParamMap = this.route.snapshot.queryParamMap;
    const previousFsPresent = queryParamMap.has("fs");
    const previousFs = queryParamMap.get("fs") === "true" ? "true" : "false";

    return {
      cameraPanelFocusPreviousFsPresent: previousFsPresent,
      cameraPanelFocusPreviousFs: previousFs,
      cameraPanelFocusAutoFs: !previousFsPresent,
    };
  }

  private focusHistoryStateForNavigation():
    | CameraPanelFocusHistoryState
    | undefined {
    return this.cameraPanelService.focusedCameraIdValue() === null
      ? undefined
      : this.readFocusHistoryState();
  }

  private readFocusHistoryState(): CameraPanelFocusHistoryState | undefined {
    const state = history.state as CameraPanelFocusHistoryState | undefined;
    if (
      state?.cameraPanelFocusPreviousFsPresent === undefined &&
      state?.cameraPanelFocusPreviousFs === undefined &&
      state?.cameraPanelFocusAutoFs === undefined
    ) {
      return undefined;
    }

    return state;
  }
}
