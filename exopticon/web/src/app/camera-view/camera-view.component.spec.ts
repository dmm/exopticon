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
import { Observable, Subscriber } from "rxjs";
import { Camera } from "../camera";
import { CameraPanelService } from "../camera-panel.service";
import { WebrtcService } from "../webrtc.service";
import { CameraViewComponent } from "./camera-view.component";

describe("CameraViewComponent", () => {
  let component: CameraViewComponent;
  let fixture: ComponentFixture<CameraViewComponent>;
  let mediaSubscriber: Subscriber<MediaStream>;
  let unsubscribed: boolean;
  let cameraPanelService: jasmine.SpyObj<CameraPanelService>;

  beforeEach(waitForAsync(() => {
    unsubscribed = false;
    const media$ = new Observable<MediaStream>((subscriber) => {
      mediaSubscriber = subscriber;
      return () => {
        unsubscribed = true;
      };
    });
    const webrtcService = jasmine.createSpyObj<WebrtcService>("WebrtcService", [
      "subscribe",
    ]);
    webrtcService.subscribe.and.returnValue(media$ as any);
    cameraPanelService = jasmine.createSpyObj<CameraPanelService>(
      "CameraPanelService",
      ["setMute"],
    );

    TestBed.configureTestingModule({
      imports: [CameraViewComponent],
      providers: [
        { provide: WebrtcService, useValue: webrtcService },
        { provide: CameraPanelService, useValue: cameraPanelService },
      ],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(CameraViewComponent);
    component = fixture.componentInstance;
    component.camera = buildCamera("front_door");
    component.enabled = true;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });

  it("unsubscribes and clears the video element on destroy", () => {
    const video = component.getVideoElement();
    const pause = spyOn(video, "pause").and.stub();
    const stream = new MediaStream();

    mediaSubscriber.next(stream);
    expect(video.srcObject).toBe(stream);

    fixture.destroy();

    expect(unsubscribed).toBeTrue();
    expect(pause).toHaveBeenCalled();
    expect(video.srcObject).toBeNull();
  });
});

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
