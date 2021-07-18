import { HttpClient } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Instant } from "@js-joda/core";
import { Interval } from "@js-joda/extra";
import { Observable } from "rxjs";
import { map } from "rxjs/operators";

interface EventDto {
  id: string;
  tag: string;
  cameraId: number;
  beginTime: string;
  endTime: string;
  observations: number[];
}

export class Event {
  readonly id: string;
  readonly tag: string;
  readonly cameraId: number;
  readonly beginTime: Instant;
  readonly endTime: Instant;
  readonly interval: Interval;
  observations: number[];

  constructor(dto: EventDto) {
    this.id = dto.id;
    this.tag = dto.tag;
    this.cameraId = dto.cameraId;
    this.beginTime = Instant.parse(dto.beginTime);
    this.endTime = Instant.parse(dto.endTime);
    this.interval = Interval.of(this.beginTime, this.endTime);
    this.observations = dto.observations;
  }
}

@Injectable({
  providedIn: "root",
})
export class EventService {
  constructor(private http: HttpClient) {}

  getEvents(): Observable<Event[]> {
    return this.http
      .get<EventDto[]>(`/v1/events`)
      .pipe(map((dtos) => dtos.map((d) => new Event(d))));
  }
}
