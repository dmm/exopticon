defmodule Exopticon.V1.CameraControllerTest do
  use ExopticonWeb.ConnCase

  @tag :integration
  test "require user authentication on all actions", %{conn: conn} do
    Enum.each(
      [
        get(conn, cameras_v1_path(conn, :index)),
        get(conn, cameras_v1_path(conn, :show, "123")),
        post(conn, cameras_v1_path(conn, :relativeMove, "134")),
        get(conn, cameras_v1_path(conn, :availability, "123"))
      ],
      fn conn ->
        assert html_response(conn, 302)
        assert conn.halted
      end
    )
  end
end
