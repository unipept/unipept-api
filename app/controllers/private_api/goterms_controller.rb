class PrivateApi::GotermsController < PrivateApi::PrivateApiController
  def goterms
    go_terms = params[:goterms] || []
    @goterms = GoTerm.where(code: go_terms)
  end
end
