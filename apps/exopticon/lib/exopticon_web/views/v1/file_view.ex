defmodule ExopticonWeb.V1.FileView do
  use ExopticonWeb, :view

  def render("index.json", %{files: files}) do
    render_many(files, ExopticonWeb.V1.FileView, "file.json")
  end

  def render("file.json", %{file: f}) do
    %{
      id: f.id,
      video_unit_id: f.video_unit_id,
      filename: f.filename,
      size: f.size
    }
  end
end
