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
    plug(ExopticonWeb.Auth, repo: Exopticon.Repo)
  end

  scope "/", ExopticonWeb do
    # Use the default browser stack
    pipe_through(:browser)

    get("/", PageController, :index)
    resources("/users", UserController)
    resources("/sessions", SessionController, only: [:new, :create, :delete])
    resources("/camera_groups", CameraGroupController)
    resources("/cameras", CameraController)
    resources("/files", FileController)

    get("/cameras/:id/playback", CameraController, :playback)
  end

  # Other scopes may use custom stacks.
  scope "/v1", ExopticonWeb do
    pipe_through(:api)

    resources("/cameras", V1.CameraController)
    post("/cameras/:id/relativeMove", V1.CameraController, :relativeMove)
    get("/files/:camera_id", V1.FileController, :index)
  end
end
