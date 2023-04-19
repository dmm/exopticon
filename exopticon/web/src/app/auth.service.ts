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
import { BehaviorSubject, Observable, of } from "rxjs";
import { catchError, map, mergeMap } from "rxjs/operators";

@Injectable({
  providedIn: "root",
})
export class AuthService {
  private loggedIn: BehaviorSubject<boolean> = new BehaviorSubject<boolean>(
    false
  );

  get isLoggedIn() {
    return this.http.get("/auth", { observe: "response" }).pipe(
      mergeMap((response) => {
        this.loggedIn.next(response.ok);
        return this.loggedIn.asObservable();
      }),
      catchError(() => this.loggedIn.asObservable())
    );
  }

  constructor(private http: HttpClient) {}

  login(username: string, password: string): Observable<boolean> {
    return this.http
      .post<Object>("/auth", {
        username: username,
        password: password,
      })
      .pipe(
        map(() => {
          this.loggedIn.next(true);
          return true;
        }),
        catchError(() => of(false))
      );
  }
}
