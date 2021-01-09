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

import { HttpClient, HttpErrorResponse } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Observable, throwError as observableThrowError } from "rxjs";
import { catchError, map } from "rxjs/operators";
import { Camera } from "./camera";

export enum PtzDirection {
  left,
  right,
  up,
  down,
}

@Injectable({
  providedIn: "root",
})
export class CameraService {
  private cameraUrl = "v1/cameras";

  constructor(private http: HttpClient) {}

  getCameras(): Observable<Camera[]> {
    return this.http.get<Camera[]>(this.cameraUrl).pipe(
      map((data) => data),
      catchError(this.handleError)
    );
  }

  getCamera(id: number | string): Observable<Camera> {
    return this.http.get<Camera[]>(this.cameraUrl).pipe(
      map((data: Camera[]) => data.find((c) => c.id == +id)),
      catchError(this.handleError)
    );
  }

  setCamera(camera: Camera): Observable<Camera> {
    return this.http
      .post<Camera>(this.cameraUrl + "/" + camera.id, camera)
      .pipe(
        map((data) => data),
        catchError(this.handleError)
      );
  }

  ptz(cameraId: number, direction: PtzDirection) {
    let directionArg: string = PtzDirection[direction];
    this.http
      .post(`${this.cameraUrl}/${cameraId}/ptz/${directionArg}`, null)
      .pipe(
        map((data) => data),
        catchError(this.handleError)
      )
      .subscribe(
        () => {},
        () => {}
      );
  }

  private handleError(res: HttpErrorResponse | any) {
    console.error(res.error || res.body.error);
    return observableThrowError(res.error || "Server error");
  }
}
