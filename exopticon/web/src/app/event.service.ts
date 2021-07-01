import { HttpClient } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Observable } from "rxjs";

export interface Event {
  id: string;
  tag: string;
  begin_time: string;
  end_time: string;
  observations: number[];
}

@Injectable({
  providedIn: "root",
})
export class EventService {
  constructor(private http: HttpClient) {}

  getEvents(): Observable<Event[]> {
    return this.http.get<Event[]>(`/v1/events`);
  }
}
