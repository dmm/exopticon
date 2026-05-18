import { HttpClient, HttpErrorResponse } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Observable, throwError as observableThrowError } from "rxjs";
import { catchError, map } from "rxjs/operators";
import { CameraGroup, CameraGroupId } from "./camera-group";

@Injectable({
  providedIn: "root",
})
export class CameraGroupService {
  private cameraGroupUrl = "v1/camera_groups";

  constructor(private http: HttpClient) {}

  getCameraGroups(): Observable<CameraGroup[]> {
    return this.http.get<CameraGroup[]>(this.cameraGroupUrl).pipe(
      map((data) => data),
      catchError(this.handleError),
    );
  }

  getCameraGroup(id: CameraGroupId): Observable<CameraGroup> {
    return this.http.get<CameraGroup>(this.cameraGroupUrl + "/" + id).pipe(
      map((data) => data),
      catchError(this.handleError),
    );
  }

  private handleError(res: HttpErrorResponse | any) {
    console.error(res.error || res.body.error);
    return observableThrowError(res.error || "Server error");
  }
}
