defmodule ExopticonWeb.PageControllerTest do
  use ExUnit.Case
  @moduletag integration: true
  use ExopticonWeb.ConnCase

  test "GET /", %{conn: conn} do
    conn = get conn, "/"
    assert html_response(conn, 200) =~ "Welcome to Phoenix!"
  end
end
