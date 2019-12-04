import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, Subject, Subscription } from 'rxjs';

import { ZonedDateTime, ZoneId } from '@js-joda/core'
import '@js-joda/timezone'

@Injectable({
  providedIn: 'root'
})
export class VideoUnitService {

  constructor(private http: HttpClient) { }

  /// Fetch video units for the specified duration.
  getVideoUnits(cameraId: number,
    beginTime: ZonedDateTime,
    endTime: ZonedDateTime): Observable<any> {
    return this.http.get(`/v1/cameras/${cameraId}/video?begin_time=${beginTime.toString()}`
      + `&end_time=${endTime.toString()}`);
  }
}
