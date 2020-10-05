import { HttpClient } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { ZonedDateTime } from "@js-joda/core";
import "@js-joda/timezone";
import { Observable } from "rxjs";
import { Observation } from "./observation";

@Injectable({
  providedIn: "root",
})
export class ObservationService {
  constructor(private http: HttpClient) {}

  getObservation(observationId: number): Observable<Observation> {
    return this.http.get<Observation>(`/v1/observations/${observationId}`);
  }

  getObservations(
    cameraId: number,
    beginTime: ZonedDateTime,
    endTime: ZonedDateTime
  ): Observable<any> {
    return this.http.get(
      `/v1/cameras/${cameraId}/observations?begin_time=${beginTime.toString()}` +
        `&end_time=${endTime.toString()}`
    );
  }
}
