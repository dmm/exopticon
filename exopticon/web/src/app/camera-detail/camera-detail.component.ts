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
import { ActivatedRoute, ParamMap } from "@angular/router";
import { Observable, of } from "rxjs";
import { switchMap } from "rxjs/operators";
import { Camera } from "../camera";
import { CameraService } from "../camera.service";

@Component({
  selector: "app-camera-detail",
  templateUrl: "./camera-detail.component.html",
  styleUrls: ["./camera-detail.component.css"],
})
export class CameraDetailComponent implements OnInit {
  public camera$: Observable<Camera>;
  public analysisConfig$: Observable<any>;

  constructor(
    public route: ActivatedRoute,
    private cameraService: CameraService
  ) {}

  ngOnInit(): void {
    this.camera$ = this.route.paramMap.pipe(
      switchMap((params: ParamMap) => {
        if (params.get("id") !== "0") {
          return this.cameraService.getCamera(+params.get("id"));
        } else {
          let cam = new Camera();
          cam.id = 0;
          cam.storageGroupId = 1;
          return of(cam);
        }
      })
    );
    this.analysisConfig$ = this.route.paramMap.pipe(
      switchMap((params: ParamMap) => {
        return this.cameraService.getCameraAnalysisConfiguration(
          +params.get("id")
        );
      })
    );
  }

  onSubmit(camera, analysisConfiguration) {
    camera.onvifPort = +camera.onvifPort;
    this.camera$ = this.cameraService.setCamera(camera);
    this.camera$.subscribe((camera) => {
      this.analysisConfig$ = this.cameraService.setCameraAnalysisConfiguration(
        camera.id,
        analysisConfiguration
      );
    });
  }
}
