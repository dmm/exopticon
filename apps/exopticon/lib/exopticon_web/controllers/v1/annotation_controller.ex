defmodule ExopticonWeb.V1.AnnotationController do
  use ExopticonWeb, :controller

  plug(:authenticate_user)

  alias Exopticon.Video
  alias Exopticon.Video.Annotation

  import ExopticonWeb.Router.Helpers

  action_fallback(ExopticonWeb.FallbackController)

  def index(conn, _params) do
    annotations = Video.list_annotations()
    render(conn, "index.json", annotations: annotations)
  end

  def create(conn, annotation_params) do
    with {:ok, %Annotation{} = annotation} <- Video.create_annotation(annotation_params) do
      conn
      |> put_status(:created)
      |> put_resp_header("location", annotation_v1_path(conn, :show, annotation))
      |> render("show.json", annotation: annotation)
    end
  end

  def show(conn, %{"id" => id}) do
    annotation = Video.get_annotation!(id)
    render(conn, "show.json", annotation: annotation)
  end

  def update(conn, %{"id" => id, "annotation" => annotation_params}) do
    annotation = Video.get_annotation!(id)

    with {:ok, %Annotation{} = annotation} <- Video.update_annotation(annotation, annotation_params) do
      render(conn, "show.json", annotation: annotation)
    end
  end

  def delete(conn, %{"id" => id}) do
    annotation = Video.get_annotation!(id)

    with {:ok, %Annotation{}} <- Video.delete_annotation(annotation) do
      send_resp(conn, :no_content, "")
    end
  end
end
