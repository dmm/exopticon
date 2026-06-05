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
  ComponentFixture,
  TestBed,
  fakeAsync,
  flushMicrotasks,
  waitForAsync,
} from "@angular/core/testing";
import { ActivatedRoute, convertToParamMap, Router } from "@angular/router";
import { of } from "rxjs";
import { CameraPanelService } from "../camera-panel.service";
import { CameraService } from "../camera.service";
import { WebrtcService } from "../webrtc.service";
import { CameraPanelComponent } from "./camera-panel.component";

describe("CameraPanelComponent", () => {
  let component: CameraPanelComponent;
  let fixture: ComponentFixture<CameraPanelComponent>;
  let cameraPanelService: jasmine.SpyObj<CameraPanelService>;
  let route: ActivatedRoute;
  let router: jasmine.SpyObj<Router>;
  let focusedCameraId: string | null;

  beforeEach(waitForAsync(() => {
    focusedCameraId = null;
    cameraPanelService = jasmine.createSpyObj<CameraPanelService>(
      "CameraPanelService",
      [
        "focusedCameraIdValue",
        "ptz",
        "setCols",
        "setDesiredCameraGroup",
        "setFocusedCamera",
        "setOffset",
        "setRows",
        "vm",
      ],
    );
    cameraPanelService.focusedCameraIdValue.and.callFake(() => focusedCameraId);
    cameraPanelService.vm.and.returnValue({
      tiles: [],
      layout: {
        columnCount: 1,
        rowCount: 1,
        itemWidthPercent: 100,
        itemHeightVh: 100,
        usesIntrinsicWidthSpacer: false,
      },
      offset: 0,
      activeCameraGroupName: "All",
      prevCameraGroupId: null,
      nextCameraGroupId: null,
      focusedCameraId: null,
      focusState: { kind: "none" },
    });

    route = buildRoute({}, {});
    router = jasmine.createSpyObj<Router>("Router", ["navigate"]);
    router.navigate.and.returnValue(Promise.resolve(true));

    TestBed.configureTestingModule({
      imports: [CameraPanelComponent],
      providers: [
        { provide: ActivatedRoute, useFactory: () => route },
        { provide: Router, useValue: router },
        { provide: CameraPanelService, useValue: cameraPanelService },
        { provide: CameraService, useValue: {} },
        { provide: WebrtcService, useValue: {} },
      ],
    })
      .overrideComponent(CameraPanelComponent, {
        set: {
          template: "",
          providers: [
            { provide: CameraPanelService, useValue: cameraPanelService },
          ],
        },
      })
      .compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(CameraPanelComponent);
    component = fixture.componentInstance;
  });

  afterEach(() => {
    history.replaceState({}, "", location.href);
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });

  it("adds focus matrix param and fs=true when focusing a camera", () => {
    route = setRouteSnapshot(
      route,
      { group: "front", rows: "2", cols: "2", offset: "4" },
      { existing: "1" },
    );

    component.focusCamera("front_door");

    expect(router.navigate).toHaveBeenCalledOnceWith(
      [
        "./",
        {
          group: "front",
          rows: "2",
          cols: "2",
          offset: "4",
          focus: "front_door",
        },
      ],
      {
        relativeTo: route,
        queryParams: { existing: "1", fs: "true" },
        replaceUrl: false,
        state: {
          cameraPanelFocusPreviousFsPresent: false,
          cameraPanelFocusPreviousFs: "false",
          cameraPanelFocusAutoFs: true,
          cameraPanelFocusPreviousScrollX: 0,
          cameraPanelFocusPreviousScrollY: 0,
        },
      },
    );
  });

  it("returns from focus by removing focus and restoring absent fs", () => {
    route = setRouteSnapshot(
      route,
      { group: "front", rows: "2", focus: "front_door" },
      { fs: "true", existing: "1" },
    );
    history.replaceState(
      {
        cameraPanelFocusPreviousFsPresent: false,
        cameraPanelFocusAutoFs: true,
      },
      "",
      location.href,
    );

    component.returnFromFocus();

    expect(router.navigate).toHaveBeenCalledOnceWith(
      ["./", { group: "front", rows: "2" }],
      {
        relativeTo: route,
        queryParams: { existing: "1" },
        replaceUrl: true,
      },
    );
  });

  it("restores the previous scroll position after returning from focus", fakeAsync(() => {
    route = setRouteSnapshot(
      route,
      { group: "front", focus: "front_door" },
      { fs: "true" },
    );
    history.replaceState(
      {
        cameraPanelFocusPreviousFsPresent: false,
        cameraPanelFocusAutoFs: true,
        cameraPanelFocusPreviousScrollX: 12,
        cameraPanelFocusPreviousScrollY: 345,
      },
      "",
      location.href,
    );
    const scrollTo = spyOn(window, "scrollTo").and.stub();
    spyOn(window, "requestAnimationFrame").and.callFake(
      (callback: FrameRequestCallback) => {
        callback(0);
        return 0;
      },
    );

    component.returnFromFocus();
    flushMicrotasks();

    expect(scrollTo).toHaveBeenCalledOnceWith({
      left: 12,
      top: 345,
      behavior: "auto",
    });
  }));

  it("normalizes a direct focused URL without fs", () => {
    route = setRouteSnapshot(route, { focus: "front_door" }, {});

    component.normalizeFocusedUrlIfNeeded();

    expect(router.navigate).toHaveBeenCalledOnceWith(
      ["./", { focus: "front_door" }],
      {
        relativeTo: route,
        queryParams: { fs: "true" },
        replaceUrl: true,
        state: {
          cameraPanelFocusPreviousFsPresent: false,
          cameraPanelFocusAutoFs: true,
        },
      },
    );
  });

  it("does not normalize a focused URL that explicitly sets fs=false", () => {
    route = setRouteSnapshot(route, { focus: "front_door" }, { fs: "false" });

    component.normalizeFocusedUrlIfNeeded();

    expect(router.navigate).not.toHaveBeenCalled();
  });

  it("preserves focus and focus history state when changing groups", () => {
    focusedCameraId = "front_door";
    route = setRouteSnapshot(
      route,
      { group: "front", focus: "front_door", rows: "2" },
      { fs: "true" },
    );
    const state = {
      cameraPanelFocusPreviousFsPresent: true,
      cameraPanelFocusPreviousFs: "false" as const,
      cameraPanelFocusAutoFs: false,
    };
    history.replaceState(state, "", location.href);

    component.setCameraGroup("back");

    expect(router.navigate).toHaveBeenCalledOnceWith(
      ["./", { group: "back", focus: "front_door", rows: "2" }],
      {
        relativeTo: route,
        queryParams: { fs: "true" },
        state,
      },
    );
  });

  it("ignores n and p while focused and exits focus on Escape", () => {
    focusedCameraId = "front_door";
    route = setRouteSnapshot(
      route,
      { focus: "front_door", offset: "3" },
      {
        fs: "true",
      },
    );
    cameraPanelService.vm.and.returnValue({
      tiles: [
        { camera: {} as any, selected: false, enabled: true, muted: true },
      ],
      layout: {
        columnCount: 1,
        rowCount: 1,
        itemWidthPercent: 100,
        itemHeightVh: 100,
        usesIntrinsicWidthSpacer: false,
      },
      offset: 3,
      activeCameraGroupName: "All",
      prevCameraGroupId: null,
      nextCameraGroupId: null,
      focusedCameraId: "front_door",
      focusState: { kind: "active", cameraId: "front_door" },
    });

    component.KeyEvent({ keyCode: 78 } as KeyboardEvent);
    component.KeyEvent({ keyCode: 80 } as KeyboardEvent);
    expect(router.navigate).not.toHaveBeenCalled();

    component.KeyEvent({ keyCode: 27 } as KeyboardEvent);
    expect(router.navigate).toHaveBeenCalledOnceWith(["./", { offset: "3" }], {
      relativeTo: route,
      queryParams: { fs: "true" },
      replaceUrl: true,
    });
  });
});

function buildRoute(
  params: Record<string, string>,
  queryParams: Record<string, string>,
): ActivatedRoute {
  return setRouteSnapshot(
    {
      paramMap: of(convertToParamMap(params)),
      queryParamMap: of(convertToParamMap(queryParams)),
    } as ActivatedRoute,
    params,
    queryParams,
  );
}

function setRouteSnapshot(
  route: ActivatedRoute,
  params: Record<string, string>,
  queryParams: Record<string, string>,
): ActivatedRoute {
  const snapshot = {
    params,
    queryParams,
    paramMap: convertToParamMap(params),
    queryParamMap: convertToParamMap(queryParams),
  };
  Object.defineProperty(route, "snapshot", {
    configurable: true,
    value: snapshot,
  });
  return route;
}
