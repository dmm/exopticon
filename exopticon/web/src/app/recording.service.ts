import { HttpClient, HttpParams } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Observable } from "rxjs";

export interface RecordingRange {
  beginTime: string;
  endTime: string;
}

export interface RecordingDescriptor {
  fileId: number;
  beginTime: string;
  endTime: string;
  byteLength: number;
  contentUrl: string;
  nextUrl: string;
}

export interface NextRecordingResponse {
  recording: RecordingDescriptor | null;
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

  resolveAt(cameraName: string, at: number): Observable<RecordingDescriptor> {
    const params = new HttpParams().set("at", new Date(at).toISOString());
    return this.http.get<RecordingDescriptor>(
      `v1/recordings/${encodeURIComponent(cameraName)}/samples`,
      { params },
    );
  }

  getNext(descriptor: RecordingDescriptor): Observable<NextRecordingResponse> {
    return this.http.get<NextRecordingResponse>(descriptor.nextUrl);
  }

  download(descriptor: RecordingDescriptor): Observable<Blob> {
    return this.http.get(descriptor.contentUrl, { responseType: "blob" });
  }
}
