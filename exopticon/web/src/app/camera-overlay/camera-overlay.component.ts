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

import { Component, Input, OnInit } from "@angular/core";
import { Camera } from "../camera";
import { CameraPanelService } from "../camera-panel.service";
import { CameraService, PtzDirection } from "../camera.service";

@Component({
  selector: "app-camera-overlay",
  templateUrl: "./camera-overlay.component.html",
  styleUrls: ["./camera-overlay.component.css"],
})
export class CameraOverlayComponent implements OnInit {
  @Input() camera: Camera;

  private directions = PtzDirection;
  private ptzActivated = false;

  constructor(
    private cameraService: CameraService,
    private cameraPanelService: CameraPanelService,
  ) {}

  ngOnInit() {}

  handleTouch() {
    // If the touch event was triggered by touching an overlay button, ignore
    // the event, otherwise toggle the overlay.
    if (!this.ptzActivated) {
      this.cameraPanelService.touchCamera(this.camera.id);
    }
    this.ptzActivated = false;
  }

  ptz(direction: PtzDirection) {
    this.ptzActivated = true;
    this.cameraService.ptz(this.camera.id, direction);
  }
}
