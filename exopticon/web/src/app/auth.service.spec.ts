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
import { EMPTY, of } from "rxjs";
import { AuthService } from "./auth.service";

describe("AuthService", () => {
  it("publishes login after the initial unauthorized request completes without a response", () => {
    const http = jasmine.createSpyObj<HttpClient>("HttpClient", [
      "get",
      "post",
    ]);
    http.get.and.returnValue(EMPTY);
    http.post.and.returnValue(of(null));
    const service = new AuthService(http);
    const states: boolean[] = [];
    const subscription = service.isLoggedIn.subscribe((loggedIn) =>
      states.push(loggedIn),
    );

    service.login("admin", "secret").subscribe();

    expect(states).toEqual([false, true]);
    subscription.unsubscribe();
  });
});
