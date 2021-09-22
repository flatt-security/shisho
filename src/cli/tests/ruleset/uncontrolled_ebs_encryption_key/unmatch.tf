resource "aws_ebs_volume" "volume" {
  availability_zone = "${var.region}a"
  size = 1
  kms_key_id = aws_kms_key.ebs_encryption.arn
}

resource "aws_ebs_volume" "volume" {
  availability_zone = "${var.region}a"
  size = 1
  kms_key_id = aws_kms_key.ebs_encryption.arn
}

resource "aws_kms_key" "ebs_encryption" {
	enable_key_rotation = true
}