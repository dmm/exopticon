import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, Subject, Subscription } from 'rxjs';

import { ZonedDateTime, ZoneId } from 'js-joda'
import 'js-joda-timezone'

@Injectable({
  providedIn: 'root'
})
export class ObservationService {

  constructor(private http: HttpClient) { }

  getObservations(cameraId: number, beginTime: ZonedDateTime, endTime: ZonedDateTime): Observable<any> {
    return this.http.get(`/v1/cameras/${cameraId}/observations?begin_time=${beginTime.toString()}`
      + `&end_time=${endTime.toString()}`);
  }
}
