defmodule ExopticonWeb.V1.AnnotationView do
  use ExopticonWeb, :view
  alias ExopticonWeb.V1.AnnotationView

  def render("index.json", %{annotations: annotations}) do
    %{data: render_many(annotations, AnnotationView, "annotation.json")}
  end

  def render("show.json", %{annotation: annotation}) do
    %{data: render_one(annotation, AnnotationView, "annotation.json")}
  end

  def render("annotation.json", %{annotation: annotation}) do
    %{
      id: annotation.id,
      key: annotation.key,
      value: annotation.value,
      source: annotation.source,
      frame_index: annotation.frame_index,
      ul_x: annotation.ul_x,
      ul_y: annotation.ul_y,
      width: annotation.width,
      height: annotation.height
    }
  end
end
