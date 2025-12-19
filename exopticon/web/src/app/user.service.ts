import { HttpClient } from "@angular/common/http";
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
import { Injectable } from "@angular/core";
import { Observable } from "rxjs";
import { map, publishReplay, refCount } from "rxjs/operators";

interface UserDto {
  id: string;
  username: string;
  timezone: string;
}

export class User {
  readonly id: string;
  readonly username: string;

  constructor(dto: UserDto) {
    this.id = dto.id;
    this.username = dto.username;
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
      refCount(),
    );
  }
}
