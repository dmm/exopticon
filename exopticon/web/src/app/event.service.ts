/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2021 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

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

  getEvents(beginTime: Instant, endTime: Instant): Observable<Event[]> {
    return this.http
      .get<EventDto[]>(`/v1/events?begin_time=${beginTime}&end_time=${endTime}`)
      .pipe(map((dtos) => dtos.map((d) => new Event(d))));
  }
}
