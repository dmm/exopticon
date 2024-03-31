/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
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
    endTime: ZonedDateTime,
  ): Observable<any> {
    return this.http.get(
      `/v1/cameras/${cameraId}/observations?begin_time=${beginTime.toString()}` +
        `&end_time=${endTime.toString()}`,
    );
  }
}
