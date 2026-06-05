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

import { ComponentFixture, TestBed, waitForAsync } from "@angular/core/testing";
import { Camera } from "../camera";
import { CameraPanelService } from "../camera-panel.service";
import { CameraService } from "../camera.service";
import { CameraOverlayComponent } from "./camera-overlay.component";

describe("CameraOverlayComponent", () => {
  let component: CameraOverlayComponent;
  let fixture: ComponentFixture<CameraOverlayComponent>;

  beforeEach(waitForAsync(() => {
    TestBed.configureTestingModule({
      imports: [CameraOverlayComponent],
      providers: [
        { provide: CameraService, useValue: {} },
        { provide: CameraPanelService, useValue: {} },
      ],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(CameraOverlayComponent);
    component = fixture.componentInstance;
    component.camera = buildCamera("front_door");
    component.muted = true;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });

  it("emits focus for the camera in normal mode", () => {
    const focusEvents: string[] = [];
    component.focusEvent.subscribe((cameraId) => focusEvents.push(cameraId));

    component.focusCamera(buildEvent());

    expect(focusEvents).toEqual(["front_door"]);
  });

  it("emits return in focus mode", () => {
    let returned = false;
    component.returnEvent.subscribe(() => {
      returned = true;
    });

    component.returnFromFocus(buildEvent());

    expect(returned).toBeTrue();
  });
});

function buildEvent(): Event {
  return {
    stopImmediatePropagation: jasmine.createSpy("stopImmediatePropagation"),
    stopPropagation: jasmine.createSpy("stopPropagation"),
  } as unknown as Event;
}

function buildCamera(name: string): Camera {
  return {
    metadata: { name, displayName: "Front Door" },
    spec: {
      storageGroupName: "default",
      ip: "127.0.0.1",
      onvifPort: 80,
      mac: "",
      username: "",
      rtspUrl: "",
      ptzType: "none",
      ptzProfileToken: "",
      ptzXStepSize: 0,
      ptzYStepSize: 0,
      enabled: true,
    },
    status: {
      phase: "running",
      active: true,
    },
  };
}
