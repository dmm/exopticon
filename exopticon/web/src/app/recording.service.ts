import { HttpClient, HttpParams } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Observable } from "rxjs";

export interface RecordingRange {
  beginTime: string;
  endTime: string;
}

@Injectable({ providedIn: "root" })
export class RecordingService {
  constructor(private http: HttpClient) {}

  getRanges(
    cameraName: string,
    begin: number,
    end: number,
  ): Observable<{ ranges: RecordingRange[] }> {
    const params = new HttpParams()
      .set("begin_time", new Date(begin).toISOString())
      .set("end_time", new Date(end).toISOString());
    return this.http.get<{ ranges: RecordingRange[] }>(
      `v1/recordings/${encodeURIComponent(cameraName)}`,
      { params },
    );
  }
}
