defmodule ExopticonWeb.AnnotationControllerTest do
  use ExopticonWeb.ConnCase

  alias Exopticon.Video
  alias Exopticon.Video.Annotation

  import ExopticonWeb.Router.Helpers

  @create_attrs %{
    frame_index: 42,
    offset: 0,
    height: 42,
    key: "some key",
    source: "some source",
    ul_x: 42,
    ul_y: 42,
    value: "some value",
    width: 42,
    hd_filename: "some value",
    sd_filename: "some value"
  }
  @update_attrs %{
    frame_index: 43,
    offset: 1,
    height: 43,
    key: "some updated key",
    source: "some updated source",
    ul_x: 43,
    ul_y: 43,
    value: "some updated value",
    width: 43,
    hd_filename: "some updated value",
    sd_filename: "some updated value"
  }
  @invalid_attrs %{
    frame_index: nil,
    offset: nil,
    height: nil,
    key: nil,
    source: nil,
    ul_x: nil,
    ul_y: nil,
    value: nil,
    width: nil,
    hd_filename: nil,
    sd_filename: nil
  }

  def fixture(:annotation) do
    {:ok, annotation} = Video.create_annotation(@create_attrs)
    annotation
  end

  setup %{conn: conn} = config do
    if username = config[:login_as] do
      user = user_fixture(username: username)
      conn = assign(conn, :current_user, user)
      conn = put_req_header(conn, "accept", "application/json")
      {:ok, conn: conn}
    else
      conn = put_req_header(conn, "accept", "application/json")
      {:ok, conn: conn}
    end
  end

  describe "index" do
    @describetag login_as: "some user"
    test "lists all annotations", %{conn: conn} do
      conn = get(conn, annotation_v1_path(conn, :index))
      assert json_response(conn, 200)["data"] == []
    end
  end

  describe "create annotation" do
    @describetag login_as: "some user"
    test "renders annotation when data is valid", %{conn: conn} do
      conn = post(conn, annotation_v1_path(conn, :create), @create_attrs)
      assert %{"id" => id} = json_response(conn, 201)["data"]

      conn = get(conn, annotation_v1_path(conn, :show, id))

      assert json_response(conn, 200)["data"] == %{
               "id" => id,
               "frame_index" => 42,
               "height" => 42,
               "key" => "some key",
               "source" => "some source",
               "ul_x" => 42,
               "ul_y" => 42,
               "value" => "some value",
               "width" => 42
             }
    end

    test "renders errors when data is invalid", %{conn: conn} do
      conn = post(conn, annotation_v1_path(conn, :create), annotation: @invalid_attrs)
      assert json_response(conn, 422)["errors"] != %{}
    end
  end

  describe "update annotation" do
    @describetag login_as: "some user"
    setup [:create_annotation]

    test "renders annotation when data is valid", %{
      conn: conn,
      annotation: %Annotation{id: id} = annotation
    } do
      conn = put(conn, annotation_v1_path(conn, :update, annotation), annotation: @update_attrs)
      assert %{"id" => ^id} = json_response(conn, 200)["data"]

      conn = get(conn, annotation_v1_path(conn, :show, id))

      assert json_response(conn, 200)["data"] == %{
               "id" => id,
               "frame_index" => 43,
               "height" => 43,
               "key" => "some updated key",
               "source" => "some updated source",
               "ul_x" => 43,
               "ul_y" => 43,
               "value" => "some updated value",
               "width" => 43
             }
    end

    test "renders errors when data is invalid", %{conn: conn, annotation: annotation} do
      conn = put(conn, annotation_v1_path(conn, :update, annotation), annotation: @invalid_attrs)
      assert json_response(conn, 422)["errors"] != %{}
    end
  end

  describe "delete annotation" do
    @describetag login_as: "some user"
    setup [:create_annotation]

    test "deletes chosen annotation", %{conn: conn, annotation: annotation} do
      conn = delete(conn, annotation_v1_path(conn, :delete, annotation))
      assert response(conn, 204)

      assert_error_sent(404, fn ->
        get(conn, annotation_v1_path(conn, :show, annotation))
      end)
    end
  end

  defp create_annotation(_) do
    annotation = fixture(:annotation)
    {:ok, annotation: annotation}
  end
end
