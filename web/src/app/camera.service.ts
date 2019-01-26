import { Injectable } from '@angular/core';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Observable, throwError as observableThrowError } from 'rxjs';
import { catchError, map } from 'rxjs/operators';

import { Camera } from './camera';

@Injectable({
  providedIn: 'root',
})

export class CameraService {
  private cameraUrl = 'v1/cameras';

  constructor(
    private http: HttpClient
  ) { }

  getCameras(): Observable<Camera[]> {
    return this.http
      .get<Camera[]>(this.cameraUrl)
      .pipe(map(data => data), catchError(this.handleError));
  }

  private handleError(res: HttpErrorResponse | any) {
    console.error(res.error || res.body.error);
    return observableThrowError(res.error || 'Server error');
  }
}
