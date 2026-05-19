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

import { computed, effect, Injectable, OnDestroy, signal } from "@angular/core";
import { concat, defer, forkJoin, fromEvent, of, Subscription } from "rxjs";
import { map } from "rxjs/operators";
import { Camera, CameraId } from "./camera";
import { ALL_GROUP_NAME, CameraGroup, CameraGroupId } from "./camera-group";
import { CameraGroupService } from "./camera-group.service";
import { CameraService, PtzDirection } from "./camera.service";
import { ActivePair, WebrtcService } from "./webrtc.service";

interface PanelCamera {
  readonly camera: Camera;
  readonly inViewport: boolean;
  readonly enabled: ActivePair;
}

enum SelectionMode {
  Touch,
  Mouse,
}

interface CameraPanelCameraProjection {
  readonly camera: Camera;
  readonly enabled: boolean;
}

interface CameraPanelProjection {
  readonly tiles: readonly CameraPanelCameraProjection[];
  readonly layout: CameraPanelLayoutViewModel;
  readonly offset: number;
  readonly activeCameraGroupName: string;
  readonly prevCameraGroupId: CameraGroupId | null;
  readonly nextCameraGroupId: CameraGroupId | null;
  readonly activeCameras: ReadonlyMap<CameraId, ActivePair>;
}

export interface CameraPanelTileViewModel {
  readonly camera: Camera;
  readonly selected: boolean;
  readonly enabled: boolean;
}

export interface CameraPanelLayoutViewModel {
  readonly columnCount: number;
  readonly rowCount: number;
  readonly itemWidthPercent: number;
  readonly itemHeightVh: number | null;
  readonly usesIntrinsicWidthSpacer: boolean;
}

export interface CameraPanelViewModel {
  readonly tiles: readonly CameraPanelTileViewModel[];
  readonly layout: CameraPanelLayoutViewModel;
  readonly offset: number;
  readonly activeCameraGroupName: string;
  readonly prevCameraGroupId: CameraGroupId | null;
  readonly nextCameraGroupId: CameraGroupId | null;
}

@Injectable()
export class CameraPanelService implements OnDestroy {
  // value between 0.0 and 1.0 inclusive representing how much of each
  // CameraView has to overlap the viewport to activate.
  readonly intersectionThreshold = 0.1;

  private readonly panelCameras = signal<ReadonlyMap<CameraId, PanelCamera>>(
    new Map(),
  );
  private readonly cameraGroups = signal<
    ReadonlyMap<CameraGroupId, CameraGroup>
  >(new Map());
  private readonly columnCount = signal(1);
  private readonly rowCount = signal(-1);
  private readonly offset = signal(0);
  private readonly selectedCameraId = signal<CameraId | null>(null);
  private selectedCameraMode = SelectionMode.Mouse;
  private readonly keyboardControlCameraId = signal<CameraId | null>(null);
  private readonly activeCameraGroupId = signal<CameraGroupId | null>(null);
  private readonly desiredCameraGroupId = signal<CameraGroupId | null>(null);
  private readonly pageVisible = signal(false);

  private readonly cameraProjection = computed(() => this.projectCameraPanel());

  readonly vm = computed<CameraPanelViewModel>(() => {
    const projection = this.cameraProjection();
    const selectedCameraId = this.selectedCameraId();

    return {
      tiles: projection.tiles.map((tile) => ({
        camera: tile.camera,
        enabled: tile.enabled,
        selected: tile.camera.metadata.name === selectedCameraId,
      })),
      layout: projection.layout,
      offset: projection.offset,
      activeCameraGroupName: projection.activeCameraGroupName,
      prevCameraGroupId: projection.prevCameraGroupId,
      nextCameraGroupId: projection.nextCameraGroupId,
    };
  });

  //
  // Public binding handlers
  //
  setMute(id: CameraId, muted: boolean) {
    const panelCameras = new Map(this.panelCameras());
    const panelCamera = panelCameras.get(id);

    if (panelCamera) {
      panelCameras.set(id, {
        ...panelCamera,
        enabled: {
          ...panelCamera.enabled,
          audio: !muted,
        },
      });
      this.panelCameras.set(panelCameras);
    }
  }

  //
  // End public event handlers
  //

  // page visible observable
  private pageVisible$ = concat(
    defer(() => of(!document.hidden)),
    fromEvent(document, "visibilitychange").pipe(map(() => !document.hidden)),
  );

  private pageVisibleSubscription: Subscription;

  private setCameraTimeout: ReturnType<typeof setTimeout> | null = null;
  private activeCameraTimeout: ReturnType<typeof setTimeout> | null = null;
  private activeCamerasToApply = new Map<CameraId, ActivePair>();

  constructor(
    private cameraService: CameraService,
    private cameraGroupService: CameraGroupService,
    private webrtcService: WebrtcService,
  ) {
    effect(() => {
      const activeCameras = new Map(this.cameraProjection().activeCameras);
      this.scheduleActiveCameraUpdate(activeCameras);
    });

    this.pageVisibleSubscription = this.pageVisible$.subscribe((visible) => {
      this.pageVisible.set(visible);
      if (visible) {
        this.setCameras();
        this.webrtcService.enable();
      } else {
        this.webrtcService.disable();
      }
    });
  }

  ngOnDestroy() {
    this.pageVisibleSubscription.unsubscribe();

    if (this.setCameraTimeout !== null) {
      clearTimeout(this.setCameraTimeout);
      this.setCameraTimeout = null;
    }

    if (this.activeCameraTimeout !== null) {
      clearTimeout(this.activeCameraTimeout);
      this.activeCameraTimeout = null;
    }
  }

  private rotateArray<T>(arr: T[], length: number): T[] {
    if (arr.length === 0) return [];
    arr = arr.slice();

    if (length > 0) {
      for (let i = 0; i < length; i++) {
        arr.unshift(arr.pop()!);
      }
    } else {
      for (let i = 0; i < Math.abs(length); i++) {
        arr.push(arr.shift()!);
      }
    }

    return arr;
  }

  private registerSetCameras() {
    if (this.setCameraTimeout == null) {
      this.setCameraTimeout = setTimeout(() => this.setCameras(), 2000);
    }
  }

  private scheduleActiveCameraUpdate(activeCameras: Map<CameraId, ActivePair>) {
    this.activeCamerasToApply = activeCameras;

    if (this.activeCameraTimeout === null) {
      this.activeCameraTimeout = setTimeout(() => {
        this.activeCameraTimeout = null;
        this.webrtcService.updateActiveCameras(this.activeCamerasToApply);
      }, 200);
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

        const existingPanelCameras = this.panelCameras();
        const panelCameras = new Map<CameraId, PanelCamera>();
        cameras
          .filter((c) => c.spec.enabled)
          .forEach((c) => {
            const existingPanelCamera = existingPanelCameras.get(
              c.metadata.name,
            );

            panelCameras.set(c.metadata.name, {
              camera: c,
              inViewport: existingPanelCamera?.inViewport ?? false,
              enabled: existingPanelCamera?.enabled ?? {
                video: true,
                audio: false,
              },
            });
          });

        const cameraGroupMap = new Map<CameraGroupId, CameraGroup>();
        cameraGroups.forEach((group) => {
          cameraGroupMap.set(group.metadata.name, group);
        });

        this.panelCameras.set(panelCameras);
        this.cameraGroups.set(cameraGroupMap);
        this.activeCameraGroupId.set(
          this.resolveCameraGroupId(this.desiredCameraGroupId()),
        );
      })
      .catch((err) => {
        console.log(err);
        this.registerSetCameras();
      });
  }

  private projectCameraPanel(): CameraPanelProjection {
    const panelCameras = this.panelCameras();
    const cameraGroups = this.cameraGroups();
    const columnCount = this.columnCount();
    const rowCount = this.rowCount();
    const offset = this.offset();
    const activeCameraGroupId = this.activeCameraGroupId();
    const cameraGroup =
      activeCameraGroupId === null
        ? undefined
        : cameraGroups.get(activeCameraGroupId);
    const layout = this.buildLayout(columnCount, rowCount);

    if (cameraGroup === undefined) {
      return {
        tiles: [],
        layout,
        offset,
        activeCameraGroupName: "All",
        prevCameraGroupId: this.prevCameraGroup(
          cameraGroups,
          activeCameraGroupId,
        ),
        nextCameraGroupId: this.nextCameraGroup(
          cameraGroups,
          activeCameraGroupId,
        ),
        activeCameras: new Map(),
      };
    }

    let groupCameras: PanelCamera[] = [];
    cameraGroup.spec.members.forEach((cameraId) => {
      const c = panelCameras.get(cameraId);
      if (c !== undefined) {
        groupCameras.push(c);
      }
    });

    let cameraCount = 0;
    if (rowCount < 1) {
      cameraCount = panelCameras.size;
    } else {
      cameraCount = rowCount * columnCount;
    }

    const cameras = this.rotateArray(
      groupCameras.map((c: PanelCamera) => c.camera),
      offset,
    ).slice(0, cameraCount);

    let activeCameras = new Map<CameraId, ActivePair>();
    const tiles = cameras.map((c) => {
      let p = panelCameras.get(c.metadata.name);
      if (p === undefined) {
        return {
          camera: c,
          enabled: false,
        };
      }
      let active =
        p.inViewport &&
        (p.enabled.video || p.enabled.audio) &&
        this.pageVisible();
      if (active) {
        activeCameras.set(c.metadata.name, { ...p.enabled });
      }

      return {
        camera: c,
        enabled: active,
      };
    });

    return {
      tiles,
      layout,
      offset,
      activeCameraGroupName: cameraGroup.metadata.displayName,
      prevCameraGroupId: this.prevCameraGroup(
        cameraGroups,
        activeCameraGroupId,
      ),
      nextCameraGroupId: this.nextCameraGroup(
        cameraGroups,
        activeCameraGroupId,
      ),
      activeCameras,
    };
  }

  private buildLayout(
    columnCount: number,
    rowCount: number,
  ): CameraPanelLayoutViewModel {
    return {
      columnCount,
      rowCount,
      itemWidthPercent: 100 / columnCount,
      itemHeightVh: rowCount > 0 ? 100 / rowCount : null,
      usesIntrinsicWidthSpacer: rowCount === -1,
    };
  }

  setRows(rowCount: number) {
    this.rowCount.set(rowCount);
  }

  setCols(colCount: number) {
    this.columnCount.set(colCount);
  }

  setOffset(offset: number) {
    this.offset.set(offset);
  }

  setCameraVisibility(
    cameraId: CameraId,
    intersectionEvents: IntersectionObserverEntry[],
  ) {
    const panelCameras = new Map(this.panelCameras());
    let panelCamera = panelCameras.get(cameraId);

    if (panelCamera !== undefined) {
      panelCameras.set(cameraId, {
        ...panelCamera,
        inViewport: intersectionEvents.some(
          (e) => e.intersectionRatio >= this.intersectionThreshold,
        ),
      });
      this.panelCameras.set(panelCameras);
    }
  }

  ptz(direction: PtzDirection) {
    const selectedCameraId = this.selectedCameraId();
    let tile = this.cameraProjection().tiles.find(
      (t) => t.camera.metadata.name === selectedCameraId,
    );
    if (tile) {
      this.cameraService.ptz(tile.camera.metadata.name, direction);
    }
  }

  // Event Handlers
  unmute(cameraId: CameraId) {}

  touchCamera(cameraId: CameraId) {
    if (this.selectedCameraId() !== cameraId) {
      this.selectedCameraId.set(cameraId);
      this.keyboardControlCameraId.set(cameraId);
      this.selectedCameraMode = SelectionMode.Touch;
    } else if (this.selectedCameraId() === cameraId) {
      this.selectedCameraId.set(null);
      this.keyboardControlCameraId.set(null);
      this.selectedCameraMode = SelectionMode.Touch;
    }
  }

  mouseOver(cameraId: CameraId) {
    if (
      this.selectedCameraId() === cameraId &&
      this.selectedCameraMode === SelectionMode.Mouse
    ) {
      // clear selected camera on second touch
      this.selectedCameraId.set(null);
    } else if (this.selectedCameraMode === SelectionMode.Mouse) {
      this.selectedCameraId.set(cameraId);
      this.keyboardControlCameraId.set(cameraId);
      this.selectedCameraMode = SelectionMode.Mouse;
    }
  }

  mouseLeave() {
    if (this.selectedCameraMode === SelectionMode.Mouse) {
      this.selectedCameraId.set(null);
      this.keyboardControlCameraId.set(null);
    }
  }

  setDesiredCameraGroup(cameraGroupId: CameraGroupId | null) {
    this.desiredCameraGroupId.set(cameraGroupId);
    this.activeCameraGroupId.set(this.resolveCameraGroupId(cameraGroupId));
  }

  private resolveCameraGroupId(
    cameraGroupId: CameraGroupId | null,
  ): CameraGroupId | null {
    const cameraGroups = this.cameraGroups();
    const desiredCameraGroupId = cameraGroupId ?? ALL_GROUP_NAME;

    if (cameraGroups.has(desiredCameraGroupId)) {
      return desiredCameraGroupId;
    }

    const activeCameraGroupId = this.activeCameraGroupId();
    if (activeCameraGroupId !== null && cameraGroups.has(activeCameraGroupId)) {
      return activeCameraGroupId;
    }

    if (cameraGroups.has(ALL_GROUP_NAME)) {
      return ALL_GROUP_NAME;
    }

    return null;
  }

  private nextCameraGroup(
    cameraGroups: ReadonlyMap<CameraGroupId, CameraGroup>,
    activeCameraGroupId: CameraGroupId | null,
  ): CameraGroupId | null {
    let ids = Array.from(cameraGroups.keys());
    if (ids.length === 0) {
      return null;
    }
    ids.sort();
    let currentIndex =
      activeCameraGroupId === null ? -1 : ids.indexOf(activeCameraGroupId);
    let nextIndex = currentIndex + 1;
    if (nextIndex >= ids.length) {
      // ALL group
      return ids[0];
    } else {
      return ids[nextIndex];
    }
  }

  private prevCameraGroup(
    cameraGroups: ReadonlyMap<CameraGroupId, CameraGroup>,
    activeCameraGroupId: CameraGroupId | null,
  ): CameraGroupId | null {
    let ids = Array.from(cameraGroups.keys());
    if (ids.length === 0) {
      return null;
    }
    ids.sort();
    ids.reverse();
    let currentIndex =
      activeCameraGroupId === null ? -1 : ids.indexOf(activeCameraGroupId);
    let prevIndex = currentIndex + 1;

    if (prevIndex >= ids.length) {
      // ALL group
      return ids[0];
    } else {
      return ids[prevIndex];
    }
  }
}
