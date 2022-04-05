/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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

import { HttpClient, HttpErrorResponse } from "@angular/common/http";
import { Injectable } from "@angular/core";
import { Instant, LocalDate, ZoneId } from "@js-joda/core";
import { Observable, throwError as observableThrowError } from "rxjs";
import { catchError, map } from "rxjs/operators";

export class Token {
  id: number;
  name: string;
  userId: number;
  expiration: LocalDate;

  constructor(dto: TokenDto) {
    this.id = dto.id;
    this.name = dto.name;
    this.userId = dto.user_id;
    this.expiration = Instant.parse(dto.expiration)
      .atZone(ZoneId.of("Z"))
      .toLocalDate();
  }
}

export class CreateUserToken {
  user_id: number;
  name: string;
  expiration: string;
}

interface TokenDto {
  id: number;
  name: string;
  user_id: number;
  expiration: string;
}

@Injectable({
  providedIn: "root",
})
export class TokenService {
  private tokenUrl = "v1/personal_access_tokens";

  constructor(private http: HttpClient) {}

  getTokens(): Observable<Token[]> {
    return this.http.get<TokenDto[]>(this.tokenUrl).pipe(
      map((data) => data.map((d) => new Token(d))),
      catchError(this.handleError)
    );
  }

  setToken(name: string, expiration: string): Observable<string> {
    let createUserToken: CreateUserToken = {
      user_id: 0,
      name: name,
      expiration: expiration,
    };
    return this.http.post<string>(this.tokenUrl, createUserToken).pipe(
      map((data) => data),
      catchError(this.handleError)
    );
  }

  deleteToken(id: number): Observable<string> {
    let url = `${this.tokenUrl}/${id}`;
    return this.http.delete<string>(url).pipe(
      map((data) => data),
      catchError(this.handleError)
    );
  }

  private handleError(res: HttpErrorResponse | any) {
    console.error(res.error || res.body.error);
    return observableThrowError(res.error || "Server error");
  }
}
