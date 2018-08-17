defmodule Exopticon.Repo.Migrations.AddOffsetToAnnotations do
  use Ecto.Migration

  def change do
    alter table(:annotations) do
      add(:offset, :integer)
    end
  end
end
