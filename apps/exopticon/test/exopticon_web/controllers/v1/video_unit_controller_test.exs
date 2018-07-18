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

  setup %{conn: conn} do
    {:ok, conn: put_req_header(conn, "accept", "application/json")}
  end

  describe "index" do
    test "lists all video_units", %{conn: conn} do
      conn = get(conn, video_unit_v1_path(conn, :index))
      #      assert json_response(conn, 200)["data"] == []
    end
  end

  describe "create video_unit" do
    @tag :integration
    test "renders video_unit when data is valid", %{conn: conn} do
      conn = post(conn, video_unit_v1_path(conn, :create), video_unit: @create_attrs)
      assert %{"id" => id} = json_response(conn, 201)["data"]

      conn = get(conn, video_unit_v1_path(conn, :show, id))

      assert json_response(conn, 200)["data"] == %{
               "id" => id,
               "begin_monotonic" => 42,
               "begin_time" => "2010-04-17 14:00:00.000000Z",
               "end_monotonic" => 42,
               "end_time" => "2010-04-17 14:00:00.000000Z",
               "monotonic_index" => 42
             }
    end

    @tag :integration
    test "renders errors when data is invalid", %{conn: conn} do
      conn = post(conn, video_unit_v1_path(conn, :create), video_unit: @invalid_attrs)
      assert json_response(conn, 422)["errors"] != %{}
    end
  end

  describe "update video_unit" do
    setup [:create_video_unit]

    @tag :integration
    test "renders video_unit when data is valid", %{
      conn: conn,
      video_unit: %VideoUnit{id: id} = video_unit
    } do
      conn = put(conn, video_unit_v1_path(conn, :update, video_unit), video_unit: @update_attrs)
      assert %{"id" => ^id} = json_response(conn, 200)["data"]

      conn = get(conn, video_unit_v1_path(conn, :show, id))

      assert json_response(conn, 200)["data"] == %{
               "id" => id,
               "begin_monotonic" => 43,
               "begin_time" => "2011-05-18 15:01:01.000000Z",
               "end_monotonic" => 43,
               "end_time" => "2011-05-18 15:01:01.000000Z",
               "monotonic_index" => 43
             }
    end

    @tag :integration
    test "renders errors when data is invalid", %{conn: conn, video_unit: video_unit} do
      conn = put(conn, video_unit_v1_path(conn, :update, video_unit), video_unit: @invalid_attrs)
      assert json_response(conn, 422)["errors"] != %{}
    end
  end

  describe "delete video_unit" do
    setup [:create_video_unit]

    @tag :integration
    test "deletes chosen video_unit", %{conn: conn, video_unit: video_unit} do
      conn = delete(conn, video_unit_v1_path(conn, :delete, video_unit))
      assert response(conn, 204)

      assert_error_sent(404, fn ->
        get(conn, video_unit_v1_path(conn, :show, video_unit))
      end)
    end
  end

  defp create_video_unit(_) do
    video_unit = fixture(:video_unit)
    {:ok, video_unit: video_unit}
  end
end
