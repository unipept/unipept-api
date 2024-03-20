class PrivateApi::EcnumbersController < PrivateApi::PrivateApiController
  def ecnumbers
    ec_nrs = params[:ecnumbers]
    @ecnumbers = EcNumber.where(code: ec_nrs)
  end
end
