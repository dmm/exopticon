defmodule ExopticonWeb.V1.VideoUnitControllerTest do
  use ExopticonWeb.ConnCase

  alias Exopticon.Video
  alias Exopticon.Video.VideoUnit

  @create_attrs %{
    begin_monotonic: 42,
    begin_time: "2010-04-17 14:00:00.000000Z",
    end_monotonic: 42,
    end_time: "2010-04-17 14:00:00.000000Z",
    monotonic_index: 42
  }
  @update_attrs %{
    begin_monotonic: 43,
    begin_time: "2011-05-18 15:01:01.000000Z",
    end_monotonic: 43,
    end_time: "2011-05-18 15:01:01.000000Z",
    monotonic_index: 43
  }
  @invalid_attrs %{
    begin_monotonic: nil,
    begin_time: nil,
    end_monotonic: nil,
    end_time: nil,
    monotonic_index: nil
  }

  def fixture(:video_unit) do
    {:ok, video_unit} = Video.create_video_unit(@create_attrs)
    video_unit
  end

  setup %{conn: conn, login_as: username} do
    user = user_fixture(username: username)
    conn = assign(conn, :current_user, user)

    {:ok, conn: put_req_header(conn, "accept", "application/json")}
  end

  describe "index" do
    @describetag login_as: "some user"
#    test "lists all video_units", %{conn: conn} do
#$      conn = get(conn, Routes.video_unit_v1_path(conn, :index))
#      assert json_response(conn, 200)["data"] == []
#    end
  end

  defp create_video_unit(_) do
    video_unit = fixture(:video_unit)
    {:ok, video_unit: video_unit}
  end
end
