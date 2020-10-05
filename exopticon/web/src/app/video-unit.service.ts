import { HttpClient } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { ZonedDateTime } from "@js-joda/core";
import "@js-joda/timezone";
import { Observable } from "rxjs";
import { map } from "rxjs/operators";
import { Observation } from "./observation";
import { VideoUnit } from "./video-unit";

@Injectable({
  providedIn: "root",
})
export class VideoUnitService {
  constructor(private http: HttpClient) {}

  getVideoUnit(videoUnitId: number): Observable<VideoUnit> {
    return this.http.get<VideoUnit>(`/v1/video_units/${videoUnitId}`);
  }

  /// Fetch video units for the specified duration.
  getVideoUnits(
    cameraId: number,
    beginTime: ZonedDateTime,
    endTime: ZonedDateTime
  ): Observable<[VideoUnit, any[], Observation[]][]> {
    return this.http
      .get<[any, any[], Observation[]][]>(
        `/v1/cameras/${cameraId}/video?begin_time=${beginTime.toString()}` +
          `&end_time=${endTime.toString()}`
      )
      .pipe(
        map((groups) => {
          return groups.map(([unit, files, obs]) => {
            return [new VideoUnit(unit), files, obs];
          });
        })
      );
  }
}
