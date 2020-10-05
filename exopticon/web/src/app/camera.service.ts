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
