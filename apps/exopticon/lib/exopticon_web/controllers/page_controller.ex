defmodule ExopticonWeb.PageController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  def index(conn, _params) do
    render(conn, "index.html")
  end

  def device_settings(conn, _params) do
    render(conn, "device_settings.html")
  end
end
