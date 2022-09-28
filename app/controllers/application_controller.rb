class ApplicationController < ActionController::API
  before_action :permit_params

  def permit_params
    params.permit!
  end
end
