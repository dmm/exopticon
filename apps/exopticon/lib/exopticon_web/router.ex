 # This file is a part of Exopticon, a free video surveillance tool. Visit
 # https://exopticon.org for more information.
 #
 # Copyright (C) 2018 David Matthew Mattli
 #
 # This program is free software: you can redistribute it and/or modify
 # it under the terms of the GNU Affero General Public License as published by
 # the Free Software Foundation, either version 3 of the License, or
 # (at your option) any later version.
 #
 # This program is distributed in the hope that it will be useful,
 # but WITHOUT ANY WARRANTY; without even the implied warranty of
 # MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 # GNU Affero General Public License for more details.
 # You should have received a copy of the GNU Affero General Public License
 # along with this program.  If not, see <https://www.gnu.org/licenses/>.

defmodule ExopticonWeb.Router do
  use ExopticonWeb, :router

  pipeline :browser do
    plug(:accepts, ["html"])
    plug(:fetch_session)
    plug(:fetch_flash)
    plug(:protect_from_forgery)
    plug(:put_secure_browser_headers)
    plug(ExopticonWeb.Auth, repo: Exopticon.Repo)
  end

  pipeline :api do
    plug(:accepts, ["json"])
    plug(:fetch_session)
    plug(:fetch_flash)
    plug(ExopticonWeb.Auth, repo: Exopticon.Repo)
  end

  scope "/", ExopticonWeb do
    # Use the default browser stack
    pipe_through(:browser)

    get("/", PageController, :index)
    get("/device_settings", PageController, :device_settings)
    resources("/users", UserController)
    resources("/sessions", SessionController, only: [:new, :create, :delete])
    resources("/camera_groups", CameraGroupController)
    resources("/cameras", CameraController)
    get("/files/browse", FileController, :browse)
    resources("/files", FileController)

    get("/cameras/:id/playback", CameraController, :playback)
  end

  # Other scopes may use custom stacks.
  scope "/v1", ExopticonWeb do
    pipe_through(:api)

    resources("/cameras", V1.CameraController, as: :cameras_v1)
    post("/cameras/:id/relativeMove", V1.CameraController, :relativeMove, as: :cameras_v1)
    get("/cameras/:id/availability", V1.CameraController, :availability, as: :cameras_v1)
    get("/files/", V1.FileController, :index)
    get("/video_units/between", V1.VideoUnitController, :between, as: :video_unit_v1)
    resources("/video_units", V1.VideoUnitController, except: [:new, :edit], as: :video_unit_v1)

  end
end
