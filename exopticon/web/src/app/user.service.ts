import { HttpClient } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { ZoneId } from "@js-joda/core";
import "@js-joda/timezone";
import { Observable } from "rxjs";
import { map, publishReplay, refCount } from "rxjs/operators";

interface UserDto {
  id: number;
  username: string;
  timezone: string;
}

export class User {
  readonly id: number;
  readonly username: string;
  readonly timezone: ZoneId;

  constructor(dto: UserDto) {
    this.id = dto.id;
    this.username = dto.username;
    this.timezone = ZoneId.of(dto.timezone);
  }
}

@Injectable({
  providedIn: "root",
})
export class UserService {
  constructor(private http: HttpClient) {}

  getUser(): Observable<User> {
    return this.http.get<UserDto>(`/v1/users/me`).pipe(
      map((dto) => new User(dto)),
      publishReplay(1),
      refCount()
    );
  }
}
