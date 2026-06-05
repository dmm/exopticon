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
  TestBed,
  fakeAsync,
  flushMicrotasks,
  tick,
} from "@angular/core/testing";
import { of } from "rxjs";
import { Camera } from "./camera";
import { ALL_GROUP_NAME, CameraGroup } from "./camera-group";
import { CameraGroupService } from "./camera-group.service";
import { CameraPanelService } from "./camera-panel.service";
import { CameraService, PtzDirection } from "./camera.service";
import { ActivePair, WebrtcService } from "./webrtc.service";

describe("CameraPanelService", () => {
  let cameraService: jasmine.SpyObj<CameraService>;
  let cameraGroupService: jasmine.SpyObj<CameraGroupService>;
  let webrtcService: jasmine.SpyObj<WebrtcService>;
  let cameras: Camera[];
  let cameraGroups: CameraGroup[];

  beforeEach(() => {
    spyOnProperty(document, "hidden", "get").and.returnValue(false);

    cameras = [
      buildCamera("front_door", "Front Door"),
      buildCamera("driveway", "Driveway"),
      buildCamera("porch", "Porch"),
      buildCamera("disabled", "Disabled", false),
    ];
    cameraGroups = [
      buildGroup(ALL_GROUP_NAME, "All", [
        "front_door",
        "driveway",
        "porch",
        "disabled",
      ]),
      buildGroup("front", "Front", ["front_door", "porch"]),
    ];

    cameraService = jasmine.createSpyObj<CameraService>("CameraService", [
      "getCameras",
      "ptz",
    ]);
    cameraGroupService = jasmine.createSpyObj<CameraGroupService>(
      "CameraGroupService",
      ["getCameraGroups"],
    );
    webrtcService = jasmine.createSpyObj<WebrtcService>("WebrtcService", [
      "enable",
      "disable",
      "updateActiveCameras",
    ]);

    cameraService.getCameras.and.callFake(() => of(cameras));
    cameraGroupService.getCameraGroups.and.callFake(() => of(cameraGroups));

    TestBed.configureTestingModule({
      providers: [
        CameraPanelService,
        { provide: CameraService, useValue: cameraService },
        { provide: CameraGroupService, useValue: cameraGroupService },
        { provide: WebrtcService, useValue: webrtcService },
      ],
    });
  });

  afterEach(() => {
    TestBed.resetTestingModule();
  });

  it("keeps normal projection unchanged when focus is not set", fakeAsync(() => {
    const service = loadService();

    service.setRows(1);
    service.setCols(2);
    service.setOffset(0);

    const vm = service.vm();
    expect(vm.focusState.kind).toBe("none");
    expect(vm.tiles.map((tile) => tile.camera.metadata.name)).toEqual([
      "front_door",
      "driveway",
    ]);
    expect(vm.layout.columnCount).toBe(2);
    expect(vm.layout.rowCount).toBe(1);
  }));

  it("returns one full-width tile for a valid focus camera", fakeAsync(() => {
    const service = loadService();
    service.setRows(1);
    service.setCols(1);
    service.setOffset(2);

    service.setFocusedCamera("porch");

    const vm = service.vm();
    expect(vm.focusState).toEqual({ kind: "active", cameraId: "porch" });
    expect(vm.tiles.length).toBe(1);
    expect(vm.tiles[0].camera.metadata.name).toBe("porch");
    expect(vm.offset).toBe(2);
    expect(vm.layout).toEqual({
      columnCount: 1,
      rowCount: 1,
      itemWidthPercent: 100,
      itemHeightVh: 100,
      usesIntrinsicWidthSpacer: false,
    });
  }));

  it("updates active cameras to only the focused camera and preserves audio", fakeAsync(() => {
    const service = loadService();
    service.setMute("porch", false);
    service.setFocusedCamera("porch");

    tick(200);

    const activeCameras = lastActiveCameraUpdate();
    expect(Array.from(activeCameras.keys())).toEqual(["porch"]);
    expect(activeCameras.get("porch")).toEqual({ video: true, audio: true });
  }));

  it("validates focus against the active group while ignoring pagination", fakeAsync(() => {
    const service = loadService();
    service.setDesiredCameraGroup("front");
    service.setRows(1);
    service.setCols(1);
    service.setOffset(0);

    service.setFocusedCamera("porch");
    expect(service.vm().focusState).toEqual({
      kind: "active",
      cameraId: "porch",
    });
    expect(service.vm().tiles[0].camera.metadata.name).toBe("porch");

    service.setFocusedCamera("driveway");
    expect(service.vm().focusState).toEqual({
      kind: "invalid",
      cameraId: "driveway",
    });
  }));

  it("does not report invalid focus until cameras and groups have loaded", fakeAsync(() => {
    const service = TestBed.inject(CameraPanelService);

    service.setFocusedCamera("missing");
    expect(service.vm().focusState).toEqual({
      kind: "pending",
      cameraId: "missing",
    });

    flushMicrotasks();
    expect(service.vm().focusState).toEqual({
      kind: "invalid",
      cameraId: "missing",
    });
  }));

  it("sends PTZ commands to the keyboard-control camera first", fakeAsync(() => {
    const service = loadService();
    service.touchCamera("front_door");
    service.setFocusedCamera("porch");

    service.ptz(PtzDirection.up);

    expect(cameraService.ptz).toHaveBeenCalledOnceWith(
      "porch",
      PtzDirection.up,
    );
  }));

  it("falls back to the selected camera when keyboard control is not projected", fakeAsync(() => {
    const service = loadService();
    service.setDesiredCameraGroup("front");
    service.setRows(1);
    service.setCols(1);
    service.touchCamera("front_door");
    service.setFocusedCamera("driveway");

    service.ptz(PtzDirection.down);

    expect(cameraService.ptz).toHaveBeenCalledOnceWith(
      "front_door",
      PtzDirection.down,
    );
  }));

  function loadService(): CameraPanelService {
    const service = TestBed.inject(CameraPanelService);
    flushMicrotasks();
    return service;
  }

  function lastActiveCameraUpdate(): Map<string, ActivePair> {
    const calls = webrtcService.updateActiveCameras.calls;
    expect(calls.count()).toBeGreaterThan(0);
    return calls.mostRecent().args[0];
  }
});

function buildCamera(
  name: string,
  displayName: string,
  enabled = true,
): Camera {
  return {
    metadata: { name, displayName },
    spec: {
      storageGroupName: "default",
      ip: "127.0.0.1",
      onvifPort: 80,
      mac: "",
      username: "",
      rtspUrl: "",
      ptzType: "relative",
      ptzProfileToken: "",
      ptzXStepSize: 0,
      ptzYStepSize: 0,
      enabled,
    },
    status: {
      phase: enabled ? "running" : "disabled",
      active: enabled,
    },
  };
}

function buildGroup(
  name: string,
  displayName: string,
  members: string[],
): CameraGroup {
  return {
    metadata: { name, displayName },
    spec: { members },
    status: {},
  };
}
