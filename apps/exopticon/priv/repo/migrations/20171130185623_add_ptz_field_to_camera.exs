defmodule Exopticon.Repo.Migrations.AddPtzFieldToCamera do
  use Ecto.Migration

  def change do
    alter table(:cameras) do
      add :ptz_type, :string
    end
  end
end
