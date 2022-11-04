import { HttpClient, HttpErrorResponse } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Observable, throwError as observableThrowError } from "rxjs";
import { catchError, map } from "rxjs/operators";
import { CameraGroup } from "./camera-group";

@Injectable({
  providedIn: "root",
})
export class CameraGroupService {
  private cameraGroupUrl = "v1/camera_groups";

  constructor(private http: HttpClient) {}

  getCameraGroups(): Observable<CameraGroup[]> {
    return this.http.get<CameraGroup[]>(this.cameraGroupUrl).pipe(
      map((data) => data),
      catchError(this.handleError)
    );
  }

  getCameraGroup(id: Number): Observable<CameraGroup> {
    return this.http.get<CameraGroup>(this.cameraGroupUrl + "/" + id).pipe(
      map((data) => data),
      catchError(this.handleError)
    );
  }

  setCameraGroup(cameraGroup: CameraGroup): Observable<CameraGroup> {
    let obs: Observable<CameraGroup>;
    if (cameraGroup.id == 0) {
      obs = this.http.post<CameraGroup>(this.cameraGroupUrl, {
        name: cameraGroup.name,
        members: cameraGroup.members,
      });
    } else {
      obs = this.http.post<CameraGroup>(
        this.cameraGroupUrl + "/" + cameraGroup.id.toString(),
        cameraGroup
      );
    }
    return obs.pipe(
      map((data) => data),
      catchError(this.handleError)
    );
  }

  deleteCameraGroup(cameraGroupId: Number): Observable<any> {
    return this.http.delete(this.cameraGroupUrl + "/" + cameraGroupId);
  }

  private handleError(res: HttpErrorResponse | any) {
    console.error(res.error || res.body.error);
    return observableThrowError(res.error || "Server error");
  }
}
