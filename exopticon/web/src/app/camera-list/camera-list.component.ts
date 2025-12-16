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

import { Component, OnInit } from "@angular/core";
import { Observable } from "rxjs";
import { Camera } from "../camera";
import { CameraService } from "../camera.service";

@Component({
    selector: "app-camera-list",
    templateUrl: "./camera-list.component.html",
    styleUrls: ["./camera-list.component.css"],
    standalone: false
})
export class CameraListComponent implements OnInit {
  cameras$: Observable<Camera[]>;

  constructor(private cameraService: CameraService) {}

  ngOnInit(): void {
    this.cameras$ = this.cameraService.getCameras();
  }
}
