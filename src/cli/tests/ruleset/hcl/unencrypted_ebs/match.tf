resource "aws_ebs_volume" "volume" {
  availability_zone = "${var.region}a"
  size = 1
}

resource "aws_ebs_volume" "volume" {
  availability_zone = "${var.region}a"
  size = 1
}