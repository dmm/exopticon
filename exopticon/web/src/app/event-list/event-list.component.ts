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

import { Component, OnInit } from "@angular/core";
import { ActivatedRoute, Router } from "@angular/router";
import { ZonedDateTime } from "@js-joda/core";
import { Observable } from "rxjs";
import { map } from "rxjs/operators";
import { Event, EventService } from "../event.service";
import { User, UserService } from "../user.service";

enum EventListState {
  Loading,
  Loaded,
}

@Component({
  selector: "app-event-list",
  templateUrl: "./event-list.component.html",
  styleUrls: ["./event-list.component.css"],
})
export class EventListComponent implements OnInit {
  public eventListState = EventListState;
  public selectedEvent: number = -1;
  public state: EventListState;
  public now = ZonedDateTime.now();

  events$: Observable<Event[]>;
  user$: Observable<User>;
  offset: number = 0;

  constructor(
    private route: ActivatedRoute,
    private router: Router,
    private eventService: EventService,
    private userService: UserService
  ) {}

  ngOnInit(): void {
    this.user$ = this.userService.getUser();
    this.refreshEvents();
    this.user$.subscribe((user) => {
      console.log("USER UPDATED");
      this.route.queryParams.subscribe((params) => {
        console.log(params);
        if (params["offset"]) {
          console.log("OFFSET UPDATED");
          this.offset = parseInt(params["offset"], 10);
          this.refreshEvents();
        }
      });
    });
    this.refreshEvents();
  }

  refreshEvents(): void {
    this.state = EventListState.Loading;
    this.selectedEvent = -1;
    this.user$.subscribe((user) => {
      this.now = ZonedDateTime.now(user.timezone).minusDays(this.offset);
      let startOfDay = this.now
        .withHour(0)
        .withMinute(0)
        .withSecond(0)
        .toInstant();
      let endOfDay = this.now
        .withHour(23)
        .withMinute(59)
        .withSecond(59)
        .toInstant();
      this.events$ = this.eventService.getEvents(startOfDay, endOfDay).pipe(
        map((events) => {
          let groups = this.groupEvents(events);
          this.state = EventListState.Loaded;
          console.log(groups);
          return groups;
        })
      );
    });
  }

  addDuration(offset: number): void {
    if (offset < 0 && this.offset === 0) {
      return;
    }
    this.offset += offset;
    this.router.navigate([], {
      relativeTo: this.route,
      queryParams: { offset: this.offset },
      queryParamsHandling: "merge",
    });
  }

  resetDuration(): void {
    this.offset = 0;
    this.router.navigate([], {
      relativeTo: this.route,
      queryParams: { offset: this.offset },
      queryParamsHandling: "merge",
    });
  }

  onEventClick(index: number): void {
    this.selectedEvent = index;
  }

  groupEventList(events: Event[]): Array<Event[]> {
    let groups = [];
    let activeGroup = new Array<Event>();

    events.forEach((e) => {
      if (activeGroup.length == 0) {
        activeGroup.push(e);
      } else {
        if (activeGroup.some((x) => x.interval.overlaps(e.interval))) {
          activeGroup.push(e);
        } else {
          groups.push(activeGroup);
          activeGroup = new Array<Event>();
        }
      }
    });

    if (activeGroup.length > 0) {
      groups.push(activeGroup);
    }
    return groups;
  }

  groupEvents(events: Event[]): Array<Event> {
    let mainGroups = [];
    let cameraGroups = events.reduce((r, a) => {
      let arr = r.has(a.cameraId) ? r.get(a.cameraId) : [];
      arr.push(a);
      r.set(a.cameraId, arr);
      return r;
    }, new Map<number, Event[]>());

    cameraGroups.forEach((value, key) => {
      let overlapping = this.groupEventList(value);
      let longestEvents = overlapping.map((ev) => {
        return ev.reduce((acc, val) => {
          if (acc == null) {
            acc = val;
          }
          return val.interval.toDuration().toMillis() >
            acc.interval.toDuration().toMillis()
            ? val
            : acc;
        }, null);
      });
      mainGroups = mainGroups.concat(longestEvents);
    });
    return mainGroups.sort((a, b) => a.beginTime.isBefore(b.beginTime));
  }
}
