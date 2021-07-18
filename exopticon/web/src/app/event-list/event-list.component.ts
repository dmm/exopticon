import { Component, OnInit } from "@angular/core";
import { Observable } from "rxjs";
import { map } from "rxjs/operators";
import { Event, EventService } from "../event.service";

@Component({
  selector: "app-event-list",
  templateUrl: "./event-list.component.html",
  styleUrls: ["./event-list.component.css"],
})
export class EventListComponent implements OnInit {
  events$: Observable<Event[]>;
  public selectedEvent: number = -1;

  constructor(private eventService: EventService) {}

  ngOnInit(): void {
    this.events$ = this.eventService.getEvents().pipe(
      map((events) => {
        let groups = this.groupEvents(events);
        console.log(groups);
        return groups;
      })
    );
  }

  onEventClick(index: number): void {
    this.selectedEvent = index;
  }

  groupEvents(events: Event[]): Array<Event> {
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

    let longestEvents = groups.map((ev) => {
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

    return longestEvents;
  }
}
